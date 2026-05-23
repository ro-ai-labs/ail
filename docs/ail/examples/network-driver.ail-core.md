# Network Driver AIL-Core Example

This pseudo-IR corresponds to `../../examples/network_driver.ail/spec.ail-spec.md`.
It is readable rather than the final canonical JSON serialization.

```text
package network-driver
profile System

node SystemComponent NetworkPacketReceiver
  Provenance: spec:network-driver

node Resource NetworkPacketReceiver.rx buffer : Buffer
node Resource NetworkPacketReceiver.packet metadata : Buffer
node Resource NetworkPacketReceiver.network device : Device

node Region NetworkPacketReceiver.packet processing region
edge NetworkPacketReceiver uses_region Region:NetworkPacketReceiver.packet processing region
edge Resource:NetworkPacketReceiver.rx buffer in_region Region:NetworkPacketReceiver.packet processing region
edge Resource:NetworkPacketReceiver.packet metadata in_region Region:NetworkPacketReceiver.packet processing region

node Capability access network device
node Capability read packet metadata
edge NetworkPacketReceiver requires Capability:access network device
edge NetworkPacketReceiver requires Capability:read packet metadata

edge NetworkPacketReceiver owns_resource Resource:NetworkPacketReceiver.rx buffer
edge NetworkPacketReceiver borrows_resource Resource:NetworkPacketReceiver.packet metadata

node Effect read network device
node Effect read packet metadata
node Effect write rx buffer
node Effect release rx buffer
edge Effect:read network device targets_resource Resource:NetworkPacketReceiver.network device
edge Effect:read packet metadata targets_resource Resource:NetworkPacketReceiver.packet metadata
edge Effect:write rx buffer targets_resource Resource:NetworkPacketReceiver.rx buffer
edge Effect:release rx buffer targets_resource Resource:NetworkPacketReceiver.rx buffer

edge NetworkPacketReceiver performs Effect:read network device
edge NetworkPacketReceiver performs Effect:read packet metadata
edge NetworkPacketReceiver performs Effect:write rx buffer
edge NetworkPacketReceiver performs Effect:release rx buffer

node Guarantee PacketStoredBeforeRelease
edge NetworkPacketReceiver guarantees Guarantee:PacketStoredBeforeRelease

node Trace PacketReceived
edge NetworkPacketReceiver records_trace Trace:PacketReceived
```
