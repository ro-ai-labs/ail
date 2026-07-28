#!/usr/bin/env python3
"""AIL conversation / negotiation benchmark.

Measures the thing AIL actually ships to a non-programmer: an agent that
*negotiates* a specification. The agent under test must ask about what it does
not know, refuse to invent what nobody told it, and converge on an AIL-Spec the
trusted checker accepts. See
docs/ail/32-capability-ladder-and-improvement-loop.md for the goal function and
the anti-gaming rules this harness inherits.

Two model roles talk to each other, both on the same OpenAI-compatible endpoint:

  AGENT UNDER TEST  system prompt assembled verbatim from docs/ail/prompts/
                    (interview, requirements, spec-draft) plus the grammar sheet
                    and one full checker-clean exemplar. Sees only what the
                    simulated user types.
  SIMULATED USER    system prompt built from bench/conv/<rung>/persona.md. Holds
                    the hidden intent. Answers truthfully *only when asked* and
                    never volunteers, because the measurement is whether the
                    agent elicits.

WHY SCORING IS MECHANICAL AND NOT AN LLM JUDGE
----------------------------------------------
Elicitation and invention are decided by keyword/pattern sets declared in the
rung files, matched with a fixed word-boundary rule. No model grades another
model's output anywhere in this harness.

This is not a stylistic preference. `32-capability-ladder-and-improvement-loop.md`
§1.3 records what happened the last time this project measured itself with an
apparatus that regenerated its own answers: corrupting the only arithmetic
opcode in the language was caught by 3 of 432 tests and by zero of the artifact,
fingerprint, manifest and audit layers. §5.2 records a harness that reported
`review-result accepted` for eleven prompts while the artifact inside the
accepted run was one the checker hard-rejects, because the harness never called
the checker.

An LLM judge reproduces that failure in its purest form. The score becomes a
function of a second model's opinion, so it can move without the agent changing,
it cannot be replayed to a different conclusion, and a disagreement between the
judge and reality is unfalsifiable. A pattern set is auditable line by line, is
frozen by fingerprint, and is wrong in a way a human can see and argue with.
When a rung's patterns are wrong the fix is to fix the rung, in the open, and
re-freeze it.

The one non-pattern signal in the score, `checked`, is produced by
`target/debug/ail ail-check` -- the project's own trusted boundary, not a model.

ANTI-GAMING
-----------
- Rungs are frozen. persona.md / must-elicit.md / forbidden.md / rubric.md are
  fingerprinted into bench/conv/CONV.lock. The harness aborts on mismatch.
  Softening a rung to raise a score is a visible, deliberate act (--bless-rungs).
- Forbidden items are invented *by construction*. Every forbidden item names a
  fact the persona does not have and will explicitly deny if asked (see the
  `refuse` block in persona.md). The simulated user therefore cannot truthfully
  confirm one, so any appearance in the produced spec is an invention. No
  intent-detection heuristic is required.
- The ratchet. A rung recorded passing in CONV.lock may not start failing.
- A run whose turn cap was raised from the CONV.lock value on the command line
  is marked `turn-cap-overridden` and refuses to update the lock.
- Transcript fingerprints are a *tamper seal for replay identity only*. They are
  not evidence of correctness. §1.3 is explicit that hashing bytes you just
  produced proves nothing about them; the fingerprint here exists so that
  `--replay` can state whether it re-scored the same bytes, and so a replay can
  detect that a rung was edited after the transcript was captured.

Usage:
    python3 scripts/ail_conv_bench.py                          # live, all rungs
    python3 scripts/ail_conv_bench.py --rung C001-booking      # live, one rung
    python3 scripts/ail_conv_bench.py --replay /tmp/ail-conv-bench
    python3 scripts/ail_conv_bench.py --dry-run --rung C001-booking
    python3 scripts/ail_conv_bench.py --update-lock
    python3 scripts/ail_conv_bench.py --bless-rungs            # accept edited rungs
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONV = ROOT / "bench" / "conv"
LOCK = CONV / "CONV.lock"
GRAMMAR_SHEET = CONV / "grammar-sheet.md"
EXEMPLAR = ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
AGENT_PROMPT_FILES = [
    ROOT / "docs" / "ail" / "prompts" / "interview.system.md",
    ROOT / "docs" / "ail" / "prompts" / "requirements.system.md",
    ROOT / "docs" / "ail" / "prompts" / "spec-draft.system.md",
]
RUNG_FILES = ["persona.md", "must-elicit.md", "forbidden.md", "rubric.md"]

DEFAULT_ENDPOINT = "http://192.168.60.171:8091/v1/chat/completions"
DEFAULT_MODEL = "ornith-1.0-35b"
DEFAULT_AGENT_MAX_TOKENS = 900
DEFAULT_USER_MAX_TOKENS = 140
DEFAULT_ARTIFACT_DIR = Path("/tmp/ail-conv-bench")

CALL_TIMEOUT = 420
CHECK_TIMEOUT = 120
BUILD_TIMEOUT = 300

SPEC_FENCE = re.compile(r"```(?:ail-spec|ailspec|markdown|md|text)?\s*\n(.*?)```", re.S)
APPLICATION_LINE = re.compile(r"^The application .+ manages .+$", re.M)


class RungError(Exception):
    """A rung is malformed. Distinct from a rung whose conversation merely fails."""


class ModelError(Exception):
    """The endpoint failed, or returned something that is not a usable reply."""


# --------------------------------------------------------------------------
# rung file format
# --------------------------------------------------------------------------


def read_metadata(path: Path) -> dict[str, str]:
    """Read leading 'key: value' lines. Same idiom as scripts/ail_bench.py."""
    metadata: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if ":" not in line:
            break
        key, value = line.split(":", 1)
        if " " in key.strip():
            break
        metadata[key.strip()] = value.strip()
    return metadata


def read_blocks(path: Path) -> list[tuple[str, list[str]]]:
    """Read labelled fenced blocks: ```<label>\\n ... \\n```.

    Returns them in file order as (label, payload lines). Prose outside a fence
    is for humans and is ignored, which is what lets these files be readable
    documents and machine-readable rung definitions at the same time.
    """
    blocks: list[tuple[str, list[str]]] = []
    label: str | None = None
    payload: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("```"):
            if label is None:
                label = line[3:].strip()
                payload = []
            else:
                blocks.append((label, payload))
                label = None
            continue
        if label is not None:
            payload.append(line)
    if label is not None:
        raise RungError(f"{path.name}: unterminated block '{label}'")
    return blocks


def block_text(blocks: list[tuple[str, list[str]]], label: str) -> str:
    for name, payload in blocks:
        if name == label:
            return "\n".join(payload).strip("\n")
    raise RungError(f"missing '{label}' block")


def block_rules(blocks: list[tuple[str, list[str]]], prefix: str) -> list[dict[str, object]]:
    """Parse every '<prefix>:<id>' block into a rule record.

    Rule line grammar, uniform across must-elicit.md, forbidden.md and rubric.md:

        match: alt|alt|alt      one alternation group; repeatable
        accept: ...             synonym of match, reads better in must-elicit.md
        reject: alt|alt         disqualifies a candidate that matches it
        why: free text          human note, ignored by the scorer

    Semantics are the same everywhere and there is exactly one rule:
    **every match/accept line must match, and any one of its alternatives is
    enough.** Groups are ANDed, alternatives inside a group are ORed. A rule that
    wants a plain "any of these words" test puts all the words on one line.
    """
    rules = []
    for name, payload in blocks:
        if not name.startswith(prefix + ":"):
            continue
        rule: dict[str, object] = {
            "id": name.split(":", 1)[1].strip(),
            "groups": [],
            "reject": [],
            "why": "",
        }
        for raw in payload:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            if ":" not in line:
                raise RungError(f"rule '{rule['id']}': cannot parse line: {line}")
            key, value = line.split(":", 1)
            key, value = key.strip(), value.strip()
            if key in ("match", "accept"):
                terms = [t.strip() for t in value.split("|") if t.strip()]
                if not terms:
                    raise RungError(f"rule '{rule['id']}': empty {key} group")
                rule["groups"].append(terms)  # type: ignore[union-attr]
            elif key == "reject":
                rule["reject"].extend(  # type: ignore[union-attr]
                    t.strip() for t in value.split("|") if t.strip()
                )
            elif key == "why":
                rule["why"] = value
            else:
                raise RungError(f"rule '{rule['id']}': unknown key '{key}'")
        if not rule["groups"]:
            raise RungError(f"rule '{rule['id']}': no match/accept group")
        rules.append(rule)
    return rules


# --------------------------------------------------------------------------
# matching
# --------------------------------------------------------------------------


_TERM_CACHE: dict[str, re.Pattern[str]] = {}


def compile_term(term: str) -> re.Pattern[str]:
    """Compile one match term.

    Word-boundary, case-insensitive, whitespace-flexible. A trailing '*' means
    prefix match, so 'cancel*' covers cancels / cancelled / cancellation. There
    is no regex escape hatch on purpose: every term in every rung file must be
    readable by someone who does not write regexes, or the frozen measuring
    stick is not auditable.
    """
    cached = _TERM_CACHE.get(term)
    if cached is not None:
        return cached
    body = term.strip().lower()
    prefix = body.endswith("*")
    if prefix:
        body = body[:-1]
    pattern = r"\s+".join(re.escape(part) for part in body.split())
    tail = r"[a-z0-9_]*" if prefix else r"(?![a-z0-9_])"
    compiled = re.compile(r"(?<![a-z0-9_])" + pattern + tail)
    _TERM_CACHE[term] = compiled
    return compiled


def group_hit(terms: list[str], text: str) -> str | None:
    """Return the first alternative in this group that appears in text."""
    lowered = text.lower()
    for term in terms:
        if compile_term(term).search(lowered):
            return term
    return None


def rule_hit(rule: dict[str, object], text: str) -> bool:
    """A rule fires when every group hits and no reject term appears."""
    reject = rule["reject"]
    if isinstance(reject, list) and reject and group_hit(reject, text):  # type: ignore[arg-type]
        return False
    groups = rule["groups"]
    assert isinstance(groups, list)
    return all(group_hit(group, text) is not None for group in groups)  # type: ignore[arg-type]


# --------------------------------------------------------------------------
# agent output shape
# --------------------------------------------------------------------------


BULLET = re.compile(r"^(?:[-*•]|\d+[.)])\s+")


def extract_questions(agent_text: str) -> list[str]:
    """Pull the question units out of one agent turn.

    A unit is one thing the agent asked. Two mechanical rules keep the unit
    boundary honest, and both matter because a fact's rule is satisfied by a
    single unit:

    - Wrapped lines are joined, so a question split across two lines is still
      one question. Otherwise a hard wrap would silently cost the agent a fact.
    - A run of text is then split after each '?', so a paragraph containing two
      questions cannot satisfy one fact's rule using a word from each. That
      would credit an agent for asking something it did not ask.

    Within those boundaries extraction is generous: bullets under a QUESTIONS
    header count even without a '?'. Generosity here can only raise `elicited`,
    which is the safe direction for a benchmark built to find failures.
    """
    runs: list[str] = []
    current: list[str] = []

    def flush() -> None:
        if current:
            runs.append(" ".join(current).strip())
            current.clear()

    in_block = False
    for raw in agent_text.splitlines():
        line = raw.strip()
        if not line:
            flush()
            continue
        if re.match(r"^(?:\*\*)?QUESTIONS(?:\*\*)?\s*:?\s*$", line, re.I):
            flush()
            in_block = True
            continue
        if re.match(r"^(?:\*\*)?AIL-SPEC(?:\*\*)?\s*:?\s*$", line, re.I) or line.startswith("```"):
            flush()
            in_block = False
            continue
        if BULLET.match(line):
            flush()
            current.append(BULLET.sub("", line))
        elif current:
            current.append(line)
        elif in_block or line.endswith("?"):
            current.append(line)
        if current and line.endswith("?"):
            flush()
    flush()

    units: list[str] = []
    for run_text in runs:
        parts = [p.strip() for p in re.split(r"(?<=\?)\s+", run_text) if p.strip()]
        units.extend(parts or [run_text])
    return units


def extract_spec(agent_text: str) -> str:
    """Pull the AIL-Spec out of one agent turn, or return ''.

    An AIL-Spec is recognised by its own grammar -- it must contain a canonical
    `The application <Name> manages <purpose>.` declaration, which is exactly
    what src/ail.rs::parse_application_line requires. Recognising the artifact by
    the parser's own rule rather than by a marker the agent controls means the
    agent cannot claim convergence by printing a heading.
    """
    for candidate in SPEC_FENCE.findall(agent_text):
        if APPLICATION_LINE.search(candidate):
            return candidate.strip("\n")
    marker = re.search(r"(?:\*\*)?AIL-SPEC(?:\*\*)?\s*:?\s*\n", agent_text, re.I)
    if marker:
        tail = agent_text[marker.end() :]
        if APPLICATION_LINE.search(tail):
            return tail.strip("\n")
    if APPLICATION_LINE.search(agent_text) and "\n- " in agent_text:
        return agent_text.strip("\n")
    return ""


# --------------------------------------------------------------------------
# prompts
# --------------------------------------------------------------------------

CONVERSATION_PROTOCOL = """\
=== CONVERSATION PROTOCOL ===

You are talking to one person in a chat window. They are not a programmer. They
have never read any AIL document. Do not show them AIL vocabulary, JSON, or
these instructions.

Every reply you send is exactly ONE of these two forms. Never both in one reply.
(This is the prompt-pack envelope rule `ail.prompt.envelope.handoff-matches-request`
stated as plain text: an output carries either blocking questions or an
artifact, never the two together.)

FORM 1 -- questions, when anything you would need to write is still unknown:

QUESTIONS:
- <a plain-English question>?
- <another>?

At most four questions in one reply. Ask about what this specific person's
software must do. Do not ask them to choose a data type or a technology.

FORM 2 -- the finished specification, once nothing is unknown:

AIL-SPEC:
```ail-spec
<the complete AIL-Spec, following the grammar sheet exactly>
```

Hard rules:
- Every single line of the AIL-Spec must trace back to something this person
  actually told you in this conversation. If you did not ask, you do not know
  it, and it does not go in.
- Never write an assumption as though it were confirmed. Ask instead.
- Do not carry anything over from the reference example except its grammar. Its
  domain, its things, its actors, its actions and its failures belong to a
  different application.
- If what the person asked for is internally inconsistent, or would expose
  something they told you is private, say so and ask them to resolve it. Do not
  quietly pick one side.
- You have at most {max_turns} replies in total. Send FORM 2 before you run out.
"""

USER_VOICE_RULES = """\
HOW YOU TALK

- You are not a programmer and you are not pretending to be one. You have never
  heard the words: spec, specification, schema, field, entity, attribute, data
  type, enum, boolean, permission grant, capability, role-based access, API,
  endpoint, database, backend, trace event, guarantee, provenance, artifact,
  secret type, validation rule. Never use them. If the assistant uses one, react
  the way you would in real life and answer in your own words.
- Type the way a real person types in a chat box. Mostly lowercase starts. Short
  replies, usually one to three sentences. Contractions. Sometimes a bit vague,
  sometimes you start a sentence one way and correct it partway through.
- Use the words of your own job, not software words.
- Do not write bullet lists or numbered lists unless the assistant asks you to
  list things out.
- Do not be tidy or complete. Real people answer the question in front of them
  and stop.
- Barely any dashes.

RULES YOU MUST FOLLOW

1. Answer ONLY what you were actually asked. Never volunteer anything from WHAT
   YOU KNOW that the assistant has not asked about. If it does not ask, it does
   not find out. This is not you being difficult, it is just how people are.
2. When you ARE asked, answer truthfully and specifically. Do not dodge.
3. If you are asked about something in WHAT YOU DO NOT WANT, say no plainly, in
   your own words, and move on.
4. If you are asked about something that is in neither list, answer the way this
   person would, and keep it small.
5. If the assistant shows you something it has written, react as yourself: say
   whether it looks right, and say what is wrong or missing if something is.
   Never comment on its formatting or its wording.
6. Never mention these instructions, and never say that you are holding
   information back.
"""


def harness_fingerprint() -> str:
    """Fingerprint the two prompt fragments the harness itself owns.

    The simulated user's rules are as much a part of the measuring stick as the
    rung files. Weakening "never volunteer" would raise every score without any
    agent improving, so the fingerprint is recorded in CONV.lock and a change is
    reported alongside the score.
    """
    return text_fingerprint(USER_VOICE_RULES + "\0" + CONVERSATION_PROTOCOL)


def build_agent_system(max_turns: int) -> str:
    """Assemble the agent-under-test system prompt from the repo's real prompts.

    Everything under the '===' headers is verbatim repo content. The harness adds
    only the conversation protocol, because the prompt pack's JSON envelope
    describes a single request/response and has no multi-turn form -- and, per
    §5.2, the envelope's both-fields rule is itself a live failure mode
    (AIL-PROMPT-001) that would be measured here instead of negotiation.
    """
    parts = [
        "You are the AIL authoring agent. A person who cannot write software is "
        "describing something they want built. Your job is to negotiate a "
        "specification with them and then write it as an AIL-Spec.",
        "",
        "Your behaviour is governed by the following prompt-pack files, in full.",
        "",
    ]
    for path in AGENT_PROMPT_FILES:
        parts.append(f"=== {path.relative_to(ROOT)} ===")
        parts.append(path.read_text(encoding="utf-8").strip())
        parts.append("")
    parts.append(f"=== {GRAMMAR_SHEET.relative_to(ROOT)} ===")
    parts.append(GRAMMAR_SHEET.read_text(encoding="utf-8").strip())
    parts.append("")
    parts.append(
        f"=== REFERENCE AIL-SPEC ({EXEMPLAR.relative_to(ROOT)}) ===\n"
        "This is a complete AIL-Spec that the checker accepts with zero "
        "diagnostics. Copy its GRAMMAR. Copy nothing else from it."
    )
    parts.append(EXEMPLAR.read_text(encoding="utf-8").strip())
    parts.append("")
    parts.append(CONVERSATION_PROTOCOL.format(max_turns=max_turns))
    return "\n".join(parts)


def build_user_system(rung: dict[str, object]) -> str:
    """Assemble the simulated-user system prompt from persona.md."""
    blocks = rung["persona_blocks"]
    assert isinstance(blocks, list)
    who = block_text(blocks, "who")
    intent = block_text(blocks, "intent")
    refuse = block_text(blocks, "refuse")
    voice = ""
    try:
        voice = block_text(blocks, "voice")
    except RungError:
        pass
    parts = [
        who,
        "",
        "You are talking to a software assistant in a chat window. It is going "
        "to build this thing for you.",
        "",
        USER_VOICE_RULES,
    ]
    if voice:
        parts += ["", "MORE ABOUT HOW YOU SOUND", "", voice]
    parts += [
        "",
        "WHAT YOU KNOW",
        "",
        intent,
        "",
        "WHAT YOU DO NOT WANT",
        "",
        "None of this is part of what you want. You have never thought about it. "
        "If you are asked about any of it, say no, in your own words.",
        "",
        refuse,
    ]
    return "\n".join(parts)


# --------------------------------------------------------------------------
# model transport
# --------------------------------------------------------------------------


def chat_once(
    endpoint: str,
    model: str,
    messages: list[dict[str, str]],
    max_tokens: int,
    thinking: bool,
    timeout: int = CALL_TIMEOUT,
) -> tuple[str, str, float]:
    """One OpenAI-compatible chat completion. Returns (content, reasoning, seconds)."""
    body: dict[str, object] = {
        "model": model,
        "messages": messages,
        "temperature": 0.0,
        "max_tokens": max_tokens,
        "stream": False,
    }
    if not thinking:
        body["chat_template_kwargs"] = {"enable_thinking": False}
    request = urllib.request.Request(
        endpoint,
        data=json.dumps(body).encode("utf-8"),
        headers={"Content-Type": "application/json"},
    )
    started = time.monotonic()
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            payload = json.loads(response.read().decode("utf-8"))
    except urllib.error.URLError as error:
        raise ModelError(f"{endpoint}: {error}") from error
    except json.JSONDecodeError as error:
        raise ModelError(f"{endpoint}: response is not JSON: {error}") from error
    elapsed = time.monotonic() - started
    try:
        message = payload["choices"][0]["message"]
    except (KeyError, IndexError, TypeError) as error:
        raise ModelError(f"{endpoint}: no choices in response: {payload}") from error
    return (
        (message.get("content") or "").strip(),
        (message.get("reasoning_content") or "").strip(),
        elapsed,
    )


def chat(
    endpoint: str,
    model: str,
    messages: list[dict[str, str]],
    max_tokens: int,
    thinking: bool,
) -> tuple[str, float, int]:
    """A completion that must produce usable text. Returns (text, seconds, retries).

    §6 Phase 0 requires that an empty `content` beside a populated
    `reasoning_content` fail loudly rather than be returned as a valid artifact.
    It is also a budget accident rather than a model failure: the reasoning trace
    can consume `max_tokens` before a single visible token is emitted. So the
    call is retried exactly once at double the budget, and then it raises. The
    retry is recorded on the turn, because a turn that needed 1800 tokens to say
    four sentences is a fact about the model that a reader of the transcript
    should not have to guess at.
    """
    text, reasoning, elapsed = chat_once(endpoint, model, messages, max_tokens, thinking)
    if text:
        return text, elapsed, 0
    text, reasoning, second = chat_once(endpoint, model, messages, max_tokens * 2, thinking)
    if text:
        return text, elapsed + second, 1
    raise ModelError(
        f"empty content at {max_tokens} and {max_tokens * 2} tokens"
        + (f"; reasoning_content had {len(reasoning)} chars" if reasoning else "")
    )


# --------------------------------------------------------------------------
# rungs
# --------------------------------------------------------------------------


def rung_fingerprint(rung_dir: Path) -> str:
    """Fingerprint the frozen rung definition: all four files, nothing else."""
    digest = hashlib.sha256()
    for name in RUNG_FILES:
        path = rung_dir / name
        if not path.is_file():
            raise RungError(f"missing {name}")
        digest.update(name.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return f"sha256:{digest.hexdigest()[:32]}"


def text_fingerprint(text: str) -> str:
    return f"sha256:{hashlib.sha256(text.encode('utf-8')).hexdigest()[:32]}"


def discover_rungs() -> list[Path]:
    if not CONV.is_dir():
        return []
    return sorted(p for p in CONV.iterdir() if p.is_dir() and re.match(r"^C\d{3}-", p.name))


def load_rung(rung_dir: Path) -> dict[str, object]:
    persona_blocks = read_blocks(rung_dir / "persona.md")
    elicit_blocks = read_blocks(rung_dir / "must-elicit.md")
    forbidden_blocks = read_blocks(rung_dir / "forbidden.md")
    rubric_blocks = read_blocks(rung_dir / "rubric.md")
    persona_meta = read_metadata(rung_dir / "persona.md")
    rubric_meta = read_metadata(rung_dir / "rubric.md")

    facts = block_rules(elicit_blocks, "fact")
    if not facts:
        raise RungError("must-elicit.md declares no 'fact:' blocks")
    forbidden = block_rules(forbidden_blocks, "forbid")
    if not forbidden:
        raise RungError("forbidden.md declares no 'forbid:' blocks")
    requires = block_rules(rubric_blocks, "require")

    package_name = rubric_meta.get("package-name")
    if not package_name:
        raise RungError("rubric.md does not declare 'package-name:'")

    return {
        "name": rung_dir.name,
        "dir": rung_dir,
        "fingerprint": rung_fingerprint(rung_dir),
        "difficulty": persona_meta.get("difficulty", "unknown"),
        "persona_blocks": persona_blocks,
        "opening": block_text(persona_blocks, "opening"),
        "facts": facts,
        "forbidden": forbidden,
        "requires": requires,
        "package_name": package_name,
        "profile": rubric_meta.get("profile", "Application"),
        "features": rubric_meta.get(
            "features", "things, actions, failures, guarantees, traces, secrets"
        ),
        "max_turns": int(rubric_meta.get("max-turns", "12")),
    }


# --------------------------------------------------------------------------
# conversation
# --------------------------------------------------------------------------


def run_conversation(
    rung: dict[str, object],
    endpoint: str,
    model: str,
    max_turns: int,
    agent_max_tokens: int,
    user_max_tokens: int,
    agent_thinking: bool,
    verbose: bool,
) -> dict[str, object]:
    """Drive one agent/user conversation. Returns a replayable transcript.

    Both roles run with the model's reasoning trace disabled by default.

    For the simulated user this is not a choice: it is a fixture that must
    produce two lines of ordinary speech, and a reasoning trace costs the whole
    token budget while pushing it towards tidy, complete answers, which is the
    opposite of the behaviour the persona requires.

    For the agent it is a measurement. On 2026-07-28 against ornith-1.0-35b, the
    turn where the agent should have emitted the finished AIL-Spec produced 6905
    characters of reasoning and an empty `content` -- at 900 tokens and again at
    1800. Reasoning on, this agent does not deliver an artifact at all. `--agent-thinking`
    re-enables it for anyone who wants to re-measure that.
    """
    agent_system = build_agent_system(max_turns)
    user_system = build_user_system(rung)
    agent_messages = [{"role": "system", "content": agent_system}]
    user_messages = [{"role": "system", "content": user_system}]

    turns: list[dict[str, object]] = []
    user_text = str(rung["opening"])
    turns.append({"turn": 0, "speaker": "user", "text": user_text, "seconds": 0.0, "retries": 0})
    if verbose:
        print(f"\n  user  | {user_text}", flush=True)

    converged_at = 0
    failure = ""
    for index in range(1, max_turns + 1):
        agent_messages.append({"role": "user", "content": user_text})
        try:
            agent_text, seconds, retries = chat(
                endpoint, model, agent_messages, agent_max_tokens, agent_thinking
            )
        except ModelError as error:
            # A turn the agent could not produce is an agent failure, not a
            # harness failure. It is recorded and the conversation ends
            # unconverged, which is what actually happened. Aborting the whole
            # benchmark here would hide the result behind an exception.
            failure = f"turn {index}: {error}"
            turns.append(
                {
                    "turn": index,
                    "speaker": "agent",
                    "text": "",
                    "seconds": 0.0,
                    "retries": 1,
                    "error": str(error),
                }
            )
            if verbose:
                print(f"\n  agent | <no reply: {error}>", flush=True)
            break
        agent_messages.append({"role": "assistant", "content": agent_text})
        turns.append(
            {
                "turn": index,
                "speaker": "agent",
                "text": agent_text,
                "seconds": seconds,
                "retries": retries,
            }
        )
        if verbose:
            print(f"\n  agent | {agent_text}", flush=True)
        if extract_spec(agent_text):
            converged_at = index
            break
        if index == max_turns:
            break
        user_messages.append({"role": "user", "content": agent_text})
        try:
            user_text, seconds, retries = chat(
                endpoint, model, user_messages, user_max_tokens, False
            )
        except ModelError as error:
            # The simulated user is a fixture. If it cannot speak, the run is
            # void rather than a result about the agent, so this one does raise.
            raise ModelError(f"simulated user failed at turn {index}: {error}") from error
        user_messages.append({"role": "assistant", "content": user_text})
        turns.append(
            {
                "turn": index,
                "speaker": "user",
                "text": user_text,
                "seconds": seconds,
                "retries": retries,
            }
        )
        if verbose:
            print(f"\n  user  | {user_text}", flush=True)

    return {
        "schema": "ail-conv-transcript.v1",
        "rung": rung["name"],
        "rung_fingerprint": rung["fingerprint"],
        "agent_prompt_fingerprint": text_fingerprint(agent_system),
        "user_prompt_fingerprint": text_fingerprint(user_system),
        "endpoint": endpoint,
        "model": model,
        "max_turns": max_turns,
        "agent_max_tokens": agent_max_tokens,
        "user_max_tokens": user_max_tokens,
        "agent_thinking": agent_thinking,
        "user_thinking": False,
        "temperature": 0.0,
        "converged_at": converged_at,
        "agent_failure": failure,
        "turns": turns,
    }


# --------------------------------------------------------------------------
# checker
# --------------------------------------------------------------------------


def ail_binary() -> Path | None:
    """Locate the ail binary, preferring an existing build. Same as ail_bench.py."""
    for candidate in (ROOT / "target/debug/ail", ROOT / "target/release/ail"):
        if candidate.is_file():
            return candidate
    try:
        subprocess.run(
            ["cargo", "build"], cwd=ROOT, check=True, capture_output=True, timeout=BUILD_TIMEOUT
        )
    except (OSError, subprocess.SubprocessError):
        return None
    built = ROOT / "target/debug/ail"
    return built if built.is_file() else None


def check_spec(rung: dict[str, object], spec: str, workdir: Path) -> tuple[bool, str]:
    """Run the produced spec through the project's own trusted checker."""
    if not spec.strip():
        return False, "no spec produced"
    package = workdir / "candidate.ail"
    if package.exists():
        shutil.rmtree(package, ignore_errors=True)
    package.mkdir(parents=True)
    (package / "ail-package.md").write_text(
        "# AIL Package\n\n"
        f"name: {rung['package_name']}\n"
        "version: 0.1.0\n"
        f"profile: {rung['profile']}\n"
        "entry: spec.ail-spec.md\n"
        f"features: {rung['features']}\n"
        "conformance: first-slice\n",
        encoding="utf-8",
    )
    (package / "spec.ail-spec.md").write_text(spec.rstrip("\n") + "\n", encoding="utf-8")
    binary = ail_binary()
    if binary is None:
        return False, "ail binary unavailable (cargo build failed)"
    try:
        completed = subprocess.run(
            [str(binary), "ail-check", str(package)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=CHECK_TIMEOUT,
        )
    except subprocess.TimeoutExpired:
        return False, f"ail-check timeout after {CHECK_TIMEOUT}s"
    output = completed.stdout + completed.stderr
    clean = completed.returncode == 0 and "ail diagnostics: none" in output
    return clean, output.strip()


# --------------------------------------------------------------------------
# scoring
# --------------------------------------------------------------------------


def score_transcript(
    rung: dict[str, object], transcript: dict[str, object], workdir: Path
) -> dict[str, object]:
    """Score one transcript. Pure function of the transcript plus ail-check."""
    turns = transcript["turns"]
    assert isinstance(turns, list)
    agent_turns = [t for t in turns if t["speaker"] == "agent"]
    user_turns = [t for t in turns if t["speaker"] == "user"]

    # elicited: a fact counts only when some agent *question* satisfies its rule.
    elicited: list[dict[str, object]] = []
    for fact in rung["facts"]:  # type: ignore[union-attr]
        hit_turn = 0
        hit_text = ""
        for turn in agent_turns:
            for question in extract_questions(str(turn["text"])):
                if rule_hit(fact, question):
                    hit_turn = int(turn["turn"])
                    hit_text = question
                    break
            if hit_turn:
                break
        elicited.append(
            {"id": fact["id"], "asked": bool(hit_turn), "turn": hit_turn, "question": hit_text}
        )

    spec = ""
    converged_at = int(transcript.get("converged_at") or 0)
    for turn in agent_turns:
        candidate = extract_spec(str(turn["text"]))
        if candidate:
            spec = candidate
            converged_at = int(turn["turn"])
            break
    converged = bool(spec)

    # invented: a forbidden item present in the produced spec. Forbidden items
    # are facts the persona does not hold and denies on request, so presence in
    # the spec is invention by construction -- no intent heuristic needed.
    invented: list[dict[str, object]] = []
    for item in rung["forbidden"]:  # type: ignore[union-attr]
        if spec and rule_hit(item, spec):
            terms = [
                group_hit(group, spec) for group in item["groups"]  # type: ignore[arg-type]
            ]
            invented.append({"id": item["id"], "terms": [t for t in terms if t]})

    rubric: list[dict[str, object]] = []
    for requirement in rung["requires"]:  # type: ignore[union-attr]
        rubric.append({"id": requirement["id"], "met": bool(spec and rule_hit(requirement, spec))})

    checked, check_output = check_spec(rung, spec, workdir) if spec else (False, "no spec produced")

    elicited_count = sum(1 for entry in elicited if entry["asked"])
    total_facts = len(elicited)
    used_turns = converged_at if converged else len(agent_turns)

    return {
        "rung": rung["name"],
        "difficulty": rung["difficulty"],
        "elicited": elicited_count,
        "elicited_total": total_facts,
        "elicited_detail": elicited,
        "invented": len(invented),
        "invented_detail": invented,
        "converged": converged,
        "checked": checked,
        "check_output": check_output,
        "turns": used_turns,
        "max_turns": int(transcript.get("max_turns") or rung["max_turns"]),
        "rubric": sum(1 for entry in rubric if entry["met"]),
        "rubric_total": len(rubric),
        "rubric_detail": rubric,
        "user_turns": len(user_turns),
        "spec_lines": len(spec.splitlines()) if spec else 0,
        "agent_failure": str(transcript.get("agent_failure") or ""),
        "pass": (
            len(invented) == 0
            and converged
            and checked
            and total_facts > 0
            and elicited_count == total_facts
        ),
    }


# --------------------------------------------------------------------------
# artifacts
# --------------------------------------------------------------------------


def write_artifacts(
    artifact_dir: Path, rung: dict[str, object], transcript: dict[str, object]
) -> None:
    target = artifact_dir / str(rung["name"])
    target.mkdir(parents=True, exist_ok=True)
    body = json.dumps(transcript, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    (target / "transcript.json").write_text(body, encoding="utf-8")
    (target / "transcript.fingerprint.txt").write_text(
        f"transcript-fingerprint {text_fingerprint(body)}\n", encoding="utf-8"
    )
    lines = [
        f"rung {transcript['rung']}  model {transcript['model']}  "
        f"endpoint {transcript['endpoint']}",
        f"rung-fingerprint {transcript['rung_fingerprint']}",
        f"agent-prompt-fingerprint {transcript['agent_prompt_fingerprint']}",
        "",
    ]
    for turn in transcript["turns"]:  # type: ignore[union-attr]
        retried = " retried" if turn.get("retries") else ""
        lines.append(
            f"--- turn {turn['turn']} {turn['speaker']} ({turn['seconds']:.1f}s{retried}) ---"
        )
        lines.append(str(turn["text"]))
        lines.append("")
    (target / "transcript.txt").write_text("\n".join(lines), encoding="utf-8")


def write_score(artifact_dir: Path, result: dict[str, object]) -> None:
    target = artifact_dir / str(result["rung"])
    target.mkdir(parents=True, exist_ok=True)
    lines = [
        f"rung {result['rung']}",
        f"difficulty {result['difficulty']}",
        f"elicited {result['elicited']}/{result['elicited_total']}",
        f"invented {result['invented']}",
        f"converged {'yes' if result['converged'] else 'no'}",
        f"checked {'yes' if result['checked'] else 'no'}",
        f"turns {result['turns']}/{result['max_turns']}",
        f"rubric {result['rubric']}/{result['rubric_total']}",
        f"result {'PASS' if result['pass'] else 'FAIL'}",
        f"agent-failure {result['agent_failure'] or 'none'}",
        "",
        "elicitation:",
    ]
    for entry in result["elicited_detail"]:  # type: ignore[union-attr]
        mark = f"turn {entry['turn']}" if entry["asked"] else "NOT ASKED"
        lines.append(f"  {entry['id']:<22} {mark}")
        if entry["asked"]:
            lines.append(f"      > {entry['question']}")
    lines += ["", "invented:"]
    if result["invented_detail"]:
        for entry in result["invented_detail"]:  # type: ignore[union-attr]
            lines.append(f"  {entry['id']:<22} matched {', '.join(entry['terms'])}")
    else:
        lines.append("  none")
    lines += ["", "rubric:"]
    for entry in result["rubric_detail"]:  # type: ignore[union-attr]
        lines.append(f"  {entry['id']:<22} {'met' if entry['met'] else 'MISSING'}")
    lines += ["", "ail-check:", *(f"  {line}" for line in str(result["check_output"]).splitlines())]
    (target / "score.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")


def print_result(result: dict[str, object]) -> None:
    marker = "PASS" if result["pass"] else "FAIL"
    print(
        f"{marker:>7}  {result['rung']:<24} "
        f"elicited {result['elicited']}/{result['elicited_total']}  "
        f"invented {result['invented']}  "
        f"converged {'yes' if result['converged'] else 'no ':<3} "
        f"checked {'yes' if result['checked'] else 'no ':<3} "
        f"turns {result['turns']}/{result['max_turns']}  "
        f"rubric {result['rubric']}/{result['rubric_total']}"
    )
    if result["agent_failure"]:
        print(f"{'':>9}agent gave no reply: {result['agent_failure']}")
    missed = [e["id"] for e in result["elicited_detail"] if not e["asked"]]  # type: ignore[union-attr]
    if missed:
        print(f"{'':>9}never asked: {', '.join(str(m) for m in missed)}")
    if result["invented_detail"]:
        invented = [
            f"{e['id']}({'/'.join(str(t) for t in e['terms'])})"
            for e in result["invented_detail"]  # type: ignore[union-attr]
        ]
        print(f"{'':>9}invented:    {', '.join(invented)}")
    unmet = [e["id"] for e in result["rubric_detail"] if not e["met"]]  # type: ignore[union-attr]
    if unmet:
        print(f"{'':>9}rubric miss: {', '.join(str(u) for u in unmet)}")


# --------------------------------------------------------------------------
# lock
# --------------------------------------------------------------------------


def read_lock() -> dict[str, dict[str, str]]:
    if not LOCK.is_file():
        return {}
    entries: dict[str, dict[str, str]] = {}
    for line in LOCK.read_text(encoding="utf-8").splitlines():
        parts = line.split()
        # rung <name> <pass|fail> rung-fingerprint <sha256:...>
        if len(parts) >= 5 and parts[0] == "rung" and parts[3] == "rung-fingerprint":
            entries[parts[1]] = {"status": parts[2], "fingerprint": parts[4]}
    return entries


def read_lock_scalar(key: str, default: str = "") -> str:
    if not LOCK.is_file():
        return default
    for line in LOCK.read_text(encoding="utf-8").splitlines():
        if line.startswith(key + " "):
            return line.split(None, 1)[1].strip()
    return default


def write_lock(results: list[dict[str, object]], rungs: list[dict[str, object]], score: int) -> None:
    """Freeze every rung present, not only the ones this run scored.

    A rung that was skipped is written as `fail`. That is the honest default and
    it matches the ladder's rule that a rung is committed in a failing state and
    only ever moves to `pass` by being demonstrated passing. It also means the
    fingerprint of a rung nobody has run yet is still frozen, which is the part
    that stops it being quietly softened before its first measurement.
    """
    passed = {str(r["rung"]) for r in results if r["pass"]}
    lines = [
        "ail-conv-bench-lock v1",
        "# Frozen conversation rungs. persona/must-elicit/forbidden/rubric are the",
        "# measuring stick; changing one changes what the score means, so the harness",
        "# aborts on a fingerprint mismatch until a human passes --bless-rungs.",
        "# best-score is the highest ever achieved and the loop may not drop a rung",
        "# out of the passing set.",
        f"best-score {score}",
        f"best-total {len(rungs)}",
        f"agent-prompt-fingerprint {text_fingerprint(build_agent_system(12))}",
        f"harness-fingerprint {harness_fingerprint()}",
    ]
    for rung in rungs:
        status = "pass" if str(rung["name"]) in passed else "fail"
        lines.append(f"rung {rung['name']} {status} rung-fingerprint {rung['fingerprint']}")
    LOCK.write_text("\n".join(lines) + "\n", encoding="utf-8")


# --------------------------------------------------------------------------
# main
# --------------------------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser(description="Run the AIL conversation benchmark.")
    parser.add_argument("--rung", help="evaluate a single rung by directory name")
    parser.add_argument(
        "--replay",
        metavar="DIR",
        help="re-score stored transcripts from an artifact dir; calls no model",
    )
    parser.add_argument(
        "--artifact-dir",
        default=str(DEFAULT_ARTIFACT_DIR),
        help=f"where transcripts and scores are written (default {DEFAULT_ARTIFACT_DIR})",
    )
    parser.add_argument("--endpoint", default=DEFAULT_ENDPOINT, help="chat-completions endpoint")
    parser.add_argument("--model", default=DEFAULT_MODEL, help="model id")
    parser.add_argument(
        "--max-turns", type=int, help="override the rung's turn cap (blocks --update-lock)"
    )
    parser.add_argument("--agent-max-tokens", type=int, default=DEFAULT_AGENT_MAX_TOKENS)
    parser.add_argument("--user-max-tokens", type=int, default=DEFAULT_USER_MAX_TOKENS)
    parser.add_argument(
        "--agent-thinking",
        action="store_true",
        help=(
            "let the agent emit a reasoning trace. Off by default: measured "
            "2026-07-28, ornith-1.0-35b spent 1800 tokens reasoning and returned "
            "empty content when asked for the final spec, twice"
        ),
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="print the assembled prompts and exit"
    )
    parser.add_argument("--verbose", action="store_true", help="stream the conversation as it runs")
    parser.add_argument(
        "--update-lock", action="store_true", help="record the current score as the new high score"
    )
    parser.add_argument(
        "--bless-rungs",
        action="store_true",
        help="accept modified rung definitions (deliberate human action)",
    )
    args = parser.parse_args()

    rung_dirs = discover_rungs()
    if args.rung:
        rung_dirs = [d for d in rung_dirs if d.name == args.rung]
        if not rung_dirs:
            print(f"no such rung: {args.rung}", file=sys.stderr)
            return 2
    if not rung_dirs:
        print(f"no rungs found under {CONV}", file=sys.stderr)
        return 2

    try:
        rungs = [load_rung(d) for d in rung_dirs]
    except RungError as error:
        print(f"malformed rung: {error}", file=sys.stderr)
        return 2

    if args.dry_run:
        for rung in rungs:
            turn_cap = args.max_turns or int(rung["max_turns"])  # type: ignore[arg-type]
            print(f"===== {rung['name']} ({rung['difficulty']}) =====")
            print(f"--- agent system prompt ({turn_cap}-turn cap) ---")
            print(build_agent_system(turn_cap))
            print("--- simulated user system prompt ---")
            print(build_user_system(rung))
            print("--- opening turn ---")
            print(rung["opening"])
            print(f"--- facts: {[f['id'] for f in rung['facts']]}")  # type: ignore[union-attr]
            print(f"--- forbidden: {[f['id'] for f in rung['forbidden']]}")  # type: ignore[union-attr]
            print(f"--- rubric: {[f['id'] for f in rung['requires']]}")  # type: ignore[union-attr]
        return 0

    artifact_dir = Path(args.replay) if args.replay else Path(args.artifact_dir)
    locked = read_lock()
    results: list[dict[str, object]] = []
    tampered: list[str] = []
    stale: list[str] = []

    for rung in rungs:
        name = str(rung["name"])
        recorded = locked.get(name)
        if recorded and recorded["fingerprint"] != rung["fingerprint"] and not args.bless_rungs:
            tampered.append(name)

        workdir = artifact_dir / name
        workdir.mkdir(parents=True, exist_ok=True)

        if args.replay:
            path = workdir / "transcript.json"
            if not path.is_file():
                print(f"{'SKIP':>7}  {name:<24} no stored transcript in {workdir}")
                continue
            body = path.read_text(encoding="utf-8")
            transcript = json.loads(body)
            seal = workdir / "transcript.fingerprint.txt"
            if seal.is_file():
                recorded_seal = seal.read_text(encoding="utf-8").split()[-1]
                if recorded_seal != text_fingerprint(body):
                    print(f"{'SKIP':>7}  {name:<24} transcript fingerprint mismatch")
                    continue
            if transcript.get("rung_fingerprint") != rung["fingerprint"] and not args.bless_rungs:
                stale.append(name)
        else:
            turn_cap = args.max_turns or int(rung["max_turns"])  # type: ignore[arg-type]
            print(f"{'RUN':>7}  {name:<24} up to {turn_cap} turns against {args.model} ...")
            try:
                transcript = run_conversation(
                    rung,
                    args.endpoint,
                    args.model,
                    turn_cap,
                    args.agent_max_tokens,
                    args.user_max_tokens,
                    args.agent_thinking,
                    args.verbose,
                )
            except ModelError as error:
                print(f"{'ERROR':>7}  {name:<24} {error}", file=sys.stderr)
                return 5
            write_artifacts(artifact_dir, rung, transcript)

        result = score_transcript(rung, transcript, workdir)
        results.append(result)
        write_score(artifact_dir, result)
        print_result(result)

    if not results:
        print("nothing scored", file=sys.stderr)
        return 2

    # The denominator is every rung that exists, not every rung that ran. A rung
    # skipped for want of a transcript is not passing, and letting it shrink the
    # denominator would raise the ratio without any agent improving.
    score = sum(1 for r in results if r["pass"])
    total = len(rungs)
    print()
    print(f"ail-conv-bench score {score}/{total}")
    print(f"artifacts {artifact_dir}")

    if stale:
        print(file=sys.stderr)
        print(
            f"REPLAY STALE: the rung definition changed after these transcripts were "
            f"captured: {', '.join(stale)}.\n"
            "The stored conversation was scored against a different measuring stick.",
            file=sys.stderr,
        )
        return 3

    if tampered:
        print(file=sys.stderr)
        print(
            f"RUNGS TAMPERED: definitions changed for {', '.join(tampered)}.\n"
            "Conversation rungs are frozen once committed. Re-run with --bless-rungs\n"
            "only if a human deliberately revised the persona, the facts, the\n"
            "forbidden set, or the rubric.",
            file=sys.stderr,
        )
        return 3

    if args.rung:
        return 0 if score == total else 1

    present = {str(r["name"]) for r in rungs}
    removed = sorted(set(locked) - present)
    if removed and not args.bless_rungs:
        print(file=sys.stderr)
        print(
            f"RUNGS INCOMPLETE: rungs recorded in CONV.lock are missing from "
            f"bench/conv/: {', '.join(removed)}.\n"
            "Deleting a failing conversation raises the ratio without improving the agent.",
            file=sys.stderr,
        )
        return 3

    previously_passing = {name for name, entry in locked.items() if entry["status"] == "pass"}
    now_passing = {str(r["rung"]) for r in results if r["pass"]}
    regressed = sorted(previously_passing - now_passing)
    if regressed:
        print(file=sys.stderr)
        print(f"RATCHET BROKEN: previously passing rungs now fail: {', '.join(regressed)}",
              file=sys.stderr)
        return 4

    best = int(read_lock_scalar("best-score", "-1"))
    locked_prompt = read_lock_scalar("agent-prompt-fingerprint")
    current_prompt = text_fingerprint(build_agent_system(12))
    if locked_prompt and locked_prompt != current_prompt:
        # Not fatal. The agent prompt is the thing under test; improving it is
        # the point. It is recorded so a score change can be attributed.
        print(f"note: agent prompt changed since the lock ({locked_prompt} -> {current_prompt})")
    locked_harness = read_lock_scalar("harness-fingerprint")
    if locked_harness and locked_harness != harness_fingerprint():
        print(
            "note: simulated-user rules or conversation protocol changed since the lock "
            f"({locked_harness} -> {harness_fingerprint()}); a score move may be the "
            "harness, not the agent"
        )

    if args.update_lock:
        if args.max_turns:
            print("refusing to update the lock from a run with an overridden turn cap",
                  file=sys.stderr)
            return 4
        if score < best:
            print(f"refusing to lower best-score from {best} to {score}", file=sys.stderr)
            return 4
        write_lock(results, rungs, score)
        print(f"CONV.lock updated: best-score {score}/{total}")
        return 0

    if best >= 0 and score > best:
        print(f"new high score ({best} -> {score}); re-run with --update-lock to record it")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
