---
schema_version: 1
open_count: 1
waived_count: 0
fixed_count: 0
total_count: 1
last_updated: 2026-10-09T23:54:52.628Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 06 | deviation | crates/archel/src/gui/qml/tabs/PkgTab.qml |  | Tab QML files bind model properties in camelCase while bridge qproperties are snake_case; debt spans ~8 tab QML files, deferred to a dedicated cleanup plan | open |  | 2026-10-09T23:54:52.628Z |  |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "06",
    "file": "crates/archel/src/gui/qml/tabs/PkgTab.qml",
    "line": null,
    "description": "Tab QML files bind model properties in camelCase while bridge qproperties are snake_case; debt spans ~8 tab QML files, deferred to a dedicated cleanup plan",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-09T23:54:52.628Z",
    "resolved_at": null,
    "milestone": null
  }
]
````
