# API Coverage — Phase 06 (gui-shell-tabs)

No external API integration: Phase 06 is a local Qt/QML GUI over the in-process
`archel-core` API. All system integrations (NetworkManager, power-profiles-daemon,
WirePlumber, BlueZ, UPower) are local D-Bus / subprocess calls already proven in
`archel-cli` during Phases 1–5; this phase adds no network/external API surface.
(The api-coverage detector fired on roadmap wording "network settings", not an
external integration.)
