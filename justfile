# Archel GUI verification harness (phase 06-09, PLAT-03 gate).
#
# `just gui-check` is the only command needed to verify the GUI crate
# from a clean checkout: Qt build + QML lint sweep + offscreen smoke
# (expects rc=124: the app must stay alive until `timeout` kills it)
# + the full gui-feature test suite (includes the gui_thin_view
# coverage audit). `just gui-check-fast` skips the qmllint sweep.
#
# QMAKE is pinned below (/usr/bin/qmake6): bare `qmake` on Arch is
# Qt 5 and breaks cxx-qt-build, so no caller may forget the pin.
#
# When `just` is not installed, mirror each recipe by hand (fixed
# literal shell lines, no arguments, no network):
#   cargo test -p archel-core --no-default-features && cargo test -p archel --no-default-features
#   QMAKE=/usr/bin/qmake6 cargo build -p archel --features gui
#   for f in $(find crates/archel/src/gui/qml -name '*.qml'); do "$(command -v qmllint-qt6 || command -v qmllint)" "$f" || exit 1; done
#   QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings; test $? -eq 124
#   QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui

# Headless workspace tests (no Qt needed).
test:
    cargo test -p archel-core --no-default-features
    cargo test -p archel --no-default-features

# Qt build of the single `archel` binary (QMAKE pin baked in).
gui-build:
    QMAKE=/usr/bin/qmake6 cargo build -p archel --features gui

# qmllint sweep over every QML file; any error fails the recipe.
lint-qml:
    for f in $(find crates/archel/src/gui/qml -name '*.qml'); do "$(command -v qmllint-qt6 || command -v qmllint)" "$f" || exit 1; done

# Offscreen smoke: the app must stay alive until `timeout` kills it
# (rc=124). Any other rc means a tab regressed at startup.
smoke: gui-build
    QT_QPA_PLATFORM=offscreen timeout 10 ./target/debug/archel settings; test $? -eq 124

# Full GUI verification in one command.
gui-check: gui-build lint-qml smoke
    QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui

# Same as gui-check but skips the qmllint sweep (fast iteration).
gui-check-fast: gui-build smoke
    QMAKE=/usr/bin/qmake6 cargo test -p archel --features gui
