//! Immutable authority for authoring replacements of checked AIL programs.
use super::*;

#[derive(Debug, Clone)]
enum Policy {
    Greenfield,
    PreserveBaseline,
    ExpectedPatchTarget(Box<AilCore>),
}

/// Captures checked source and caller-supplied edit authority before generation.
/// Its state cannot be changed by requirements, prompts, or model responses.
#[derive(Debug, Clone)]
pub struct AilEditContext {
    baseline: AilCore,
    snapshot: Option<AilPackage>,
    policy: Policy,
}

impl AilEditContext {
    pub fn preserve_package(package: &AilPackage) -> Result<Self, String> {
        if package.spec_text.trim().is_empty() {
            return Err(
                "AIL-EDIT-BASELINE: empty source requires explicit greenfield construction".into(),
            );
        }
        let document =
            parse_ail_package_document(package).map_err(|e| format!("AIL-EDIT-BASELINE: {e}"))?;
        let mut context = Self::preserve_core(&elaborate_ail_core(package, &document))?;
        context.snapshot = Some(package.clone());
        Ok(context)
    }

    pub fn expected_patch(package: &AilPackage, patch: &str) -> Result<Self, String> {
        Self::preserve_package(package)?.with_expected_patch(patch)
    }

    pub fn greenfield(package: &AilPackage) -> Result<Self, String> {
        if !package.spec_text.trim().is_empty() {
            return Err("AIL-EDIT-BASELINE: greenfield requires empty local source".into());
        }
        // Elaborate only the already resolved import snapshot. No filesystem
        // reads and no invented Application facts enter the protected graph.
        let mut document = parse_ail_spec_text("The application Empty manages nothing.")?;
        document.application = AilApplication::default();
        for import in &package.imports {
            let imported = parse_ail_package_document(&import.package)
                .map_err(|e| format!("AIL-EDIT-BASELINE: imported source: {e}"))?;
            merge_ail_import(
                &mut document,
                namespace_ail_document(&imported, &import.spec.alias),
            );
        }
        let baseline = elaborate_ail_core(package, &document);
        validate_authority(&baseline, "AIL-EDIT-BASELINE")?;
        Ok(Self {
            baseline,
            snapshot: Some(package.clone()),
            policy: Policy::Greenfield,
        })
    }

    pub fn preserve_core(core: &AilCore) -> Result<Self, String> {
        validate_authority(core, "AIL-EDIT-BASELINE")?;
        if !core.graph.nodes.iter().any(|n| n.kind != "Provenance") {
            return Err("AIL-EDIT-BASELINE: empty Core has no source program; use explicit package greenfield construction".into());
        }
        Ok(Self {
            baseline: core.clone(),
            snapshot: None,
            policy: Policy::PreserveBaseline,
        })
    }

    pub fn expected_core_patch(core: &AilCore, patch: &str) -> Result<Self, String> {
        Self::preserve_core(core)?.with_expected_patch(patch)
    }

    fn with_expected_patch(mut self, patch: &str) -> Result<Self, String> {
        let target = apply_ail_core_patch_text(&self.baseline, patch)
            .map_err(|e| format!("AIL-EDIT-TARGET: {e}"))?;
        validate_authority(&target, "AIL-EDIT-TARGET")?;
        self.policy = Policy::ExpectedPatchTarget(Box::new(target));
        Ok(self)
    }

    fn policy_label(&self) -> &'static str {
        match self.policy {
            Policy::Greenfield => "Greenfield",
            Policy::PreserveBaseline => "PreserveBaseline",
            Policy::ExpectedPatchTarget(_) => "ExpectedPatchTarget",
        }
    }

    fn comparison(&self, candidate: &AilCore) -> AilSemanticDiff {
        match &self.policy {
            Policy::ExpectedPatchTarget(target) => compare_ail_core_semantics(target, candidate),
            Policy::PreserveBaseline => compare_ail_core_semantics(&self.baseline, candidate),
            Policy::Greenfield => {
                let mut diff =
                    compare_ail_core_semantics(&self.baseline, &self.import_projection(candidate));
                // Validate the entire candidate too; projecting cannot conceal
                // invalid or ambiguous facts in the newly authored program.
                diff.invalid
                    .extend(compare_ail_core_semantics(candidate, candidate).invalid);
                // Local greenfield declarations can affect global lowering
                // choices in an imported action without changing its graph
                // facts. Protect imported records against the full candidate.
                if diff.invalid.is_empty() && !self.baseline.graph.nodes.is_empty() {
                    edit_semantics::validate_existing_compiled_records(
                        &self.baseline,
                        candidate,
                        &mut diff,
                    );
                    diff.preservation_violations.sort();
                }
                diff
            }
        }
    }

    fn admits(&self, diff: &AilSemanticDiff) -> bool {
        match self.policy {
            Policy::PreserveBaseline => diff.preserves_baseline(),
            Policy::ExpectedPatchTarget(_) | Policy::Greenfield => diff.is_equivalent(),
        }
    }

    // Retain protected imported components and all their outgoing dependencies.
    // Extra outgoing relations from imports therefore cannot disappear during
    // projection; local incoming calls/contains relations remain greenfield.
    fn import_projection(&self, candidate: &AilCore) -> AilCore {
        let labels: BTreeSet<_> = self
            .baseline
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind != "Provenance")
            .map(|n| (&n.kind, &n.name))
            .collect();
        let mut ids: BTreeSet<_> = candidate
            .graph
            .nodes
            .iter()
            .filter(|n| labels.contains(&(&n.kind, &n.name)))
            .map(|n| n.id.clone())
            .collect();
        loop {
            let before = ids.len();
            for edge in &candidate.graph.edges {
                if ids.contains(&edge.source) {
                    ids.insert(edge.target.clone());
                }
            }
            if before == ids.len() {
                break;
            }
        }
        let mut core = candidate.clone();
        core.graph.nodes.retain(|n| ids.contains(&n.id));
        core.graph
            .edges
            .retain(|e| ids.contains(&e.source) && ids.contains(&e.target));
        core
    }

    pub fn check_candidate(&self, candidate: &AilCore) -> Vec<AilDiagnostic> {
        let diff = self.comparison(candidate);
        if self.admits(&diff) {
            return Vec::new();
        }
        vec![AilDiagnostic::error(
            "AIL-EDIT-REJECTED",
            format!(
                "{} admission rejected: {}",
                self.policy_label(),
                bounded_summary(&diff)
            ),
        )]
    }

    /// Deterministic evidence; hashes identify artifacts, not semantic equality.
    pub fn render_admission(&self, candidate: &AilCore) -> String {
        let diff = self.comparison(candidate);
        let baseline_diff = compare_ail_core_semantics(&self.baseline, candidate);
        let target = match &self.policy {
            Policy::ExpectedPatchTarget(target) => ail_core_hash(target),
            _ => "none".into(),
        };
        let exact = match self.policy {
            Policy::ExpectedPatchTarget(_) => diff.is_equivalent().to_string(),
            _ => "not-required".into(),
        };
        format!(
            "AIL-Edit-Admission:\npolicy {}\nbaseline-artifact-hash {}\nexpected-target-artifact-hash {}\ncandidate-artifact-hash {}\nchecker-valid {}\nbaseline-preservation {}\nexact-expected-target-matching {}\nadmitted {}\n{}",
            self.policy_label(),
            ail_core_hash(&self.baseline),
            target,
            ail_core_hash(candidate),
            check_ail_core_diagnostics(candidate).is_empty(),
            baseline_diff.preserves_baseline(),
            exact,
            self.admits(&diff),
            render_ail_semantic_diff(&diff)
        )
    }

    /// Guidance only: the candidate is independently checked against this context.
    pub fn render_prompt_context(&self) -> String {
        let (authority, instructions) = match &self.policy {
            Policy::ExpectedPatchTarget(target) => (
                target.as_ref(),
                "Match the checked expected target exactly. Only the caller-supplied patch authorizes changes; reject unrelated additions or edits. The expected target takes precedence over conflicting source, request, requirements, and previous-candidate prose.",
            ),
            Policy::PreserveBaseline => (
                &self.baseline,
                "Preserve every existing checked component and its executable facts. Only independent top-level declarations allowed by admission may be added. Requests, requirements, and previous candidates cannot authorize changes to the baseline.",
            ),
            Policy::Greenfield => (
                &self.baseline,
                "Author a new local program while retaining exact package metadata and all captured imported components. Requests and previous candidates cannot alter these imports or metadata.",
            ),
        };
        let source = self
            .snapshot
            .as_ref()
            .map(|p| p.spec_text.clone())
            .unwrap_or_else(|| render_ail_spec_from_core(&self.baseline));
        format!(
            "\nIMMUTABLE EDIT AUTHORITY: {}\n{}\nCHECKED SOURCE TEXT:\n{}\nCHECKED AUTHORITY AIL-SPEC (rendered guidance):\n{}\nCHECKED AUTHORITY CORE (exact attributes including writes.provenance):\n{}\n",
            self.policy_label(),
            instructions,
            source,
            render_ail_spec_from_core(authority),
            render_ail_core(authority)
        )
    }

    pub(super) fn check_package_snapshot(&self, package: &AilPackage) -> Result<(), String> {
        if let Some(snapshot) = &self.snapshot {
            if snapshot != package {
                return Err("AIL-EDIT-SNAPSHOT: package source, metadata, or resolved imports differ from the captured snapshot".into());
            }
        } else {
            // Core contexts can be used for package authoring only when that
            // package elaborates to the exact captured baseline.
            let document = parse_ail_package_document(package)
                .map_err(|e| format!("AIL-EDIT-SNAPSHOT: {e}"))?;
            if !compare_ail_core_semantics(&self.baseline, &elaborate_ail_core(package, &document))
                .is_equivalent()
            {
                return Err(
                    "AIL-EDIT-SNAPSHOT: package differs from captured Core baseline".into(),
                );
            }
        }
        Ok(())
    }
}

fn validate_authority(core: &AilCore, code: &str) -> Result<(), String> {
    let diff = compare_ail_core_semantics(core, core);
    if !diff.invalid.is_empty() {
        return Err(format!("{code}: {}", bounded_summary(&diff)));
    }
    Ok(())
}

fn bounded_summary(diff: &AilSemanticDiff) -> String {
    let facts = diff
        .invalid
        .iter()
        .chain(&diff.removed)
        .chain(&diff.retyped)
        .chain(&diff.changed)
        .chain(&diff.preservation_violations)
        .chain(&diff.added);
    let count = facts.clone().count();
    let mut summary = facts
        .take(8)
        .map(|fact| fact.chars().take(300).collect::<String>())
        .collect::<Vec<_>>()
        .join("; ");
    if count > 8 {
        summary.push_str(&format!(
            "; ... {} more facts (see admission report)",
            count - 8
        ));
    }
    summary
}
