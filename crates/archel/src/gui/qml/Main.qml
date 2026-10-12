// Archel settings shell (D-22): 200px vertical rail of 10 icon+label
// TabButtons over a content StackLayout, replacing the Phase-1 spike.
//
// Pattern (proven in Spike.qml, retired by this file): QtQuick.Controls
// TabBar is horizontal-only, so vertical tabs are a Column of TabButtons
// driving a StackLayout currentIndex. The Column lives in its own
// Flickable so ten buttons taller than the window scroll instead of
// clipping (UI-SPEC rail overflow row). Tab labels are French in the
// locked order (D-25); 16x16 vendored Material-style icons (MIT) sit
// beside each label (D-25). The frame restores from ShellModel
// (geometry.rs, D-23), validated against the live screens at
// Component.onCompleted and saved on onClosing.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import org.archel.settings

ApplicationWindow {
    id: window
    visible: true
    x: shell.launchX
    y: shell.launchY
    width: shell.launchWidth
    height: shell.launchHeight
    minimumWidth: 720
    minimumHeight: 480
    title: "Archel Réglages"
    visibility: shell.launchMaximized ? Window.Maximized : Window.Auto
    color: theme.colorWindow
    palette.window: theme.colorWindow
    palette.base: theme.colorBase
    palette.alternateBase: theme.colorAlternateBase
    palette.mid: theme.colorMid
    palette.button: theme.colorButton
    palette.buttonText: theme.colorText
    palette.text: theme.colorText
    palette.windowText: theme.colorText
    palette.brightText: theme.colorBrightText
    palette.placeholderText: theme.colorPlaceholderText
    palette.accent: theme.colorAccent
    palette.highlight: theme.colorHighlight
    palette.highlightedText: theme.colorHighlightedText

    // Geometry bridge (bridge.rs). Defaults mirror the retired QML
    // literals, so the window is correct even before validation runs.
    ShellModel {
        id: shell
    }

    // Global theme: every palette role of the window binds to the
    // ThemeModel, and Qt propagates the palette to all controls and
    // popups (spec 2026-10-11). A mode/accent change is one Rust
    // property write; every binding below re-evaluates.
    ThemeModel {
        id: theme
    }

    // Validate the saved frame against the real screens as soon as the
    // window exists (screens marshalled as JSON; no QScreen in the bridge).
    Component.onCompleted: {
        var rects = []
        for (var i = 0; i < Qt.application.screens.length; i++) {
            var s = Qt.application.screens[i]
            rects.push({ x: s.virtualX, y: s.virtualY, width: s.width, height: s.height })
        }
        shell.validateStartup(JSON.stringify(rects))
    }

    // Persist the frame on close (failures swallowed Rust-side, D-23).
    onClosing: shell.saveGeometry(
        window.x, window.y, window.width, window.height,
        window.visibility === Window.Maximized)

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Rail: fixed 200px, scrolling its own Flickable (UI-SPEC rail).
        // Style C (spec): active tab tinted with the accent, hover uses
        // the elevated surface, rounded 8px, 36px tall.
        Flickable {
            Layout.preferredWidth: 200
            Layout.fillHeight: true
            contentHeight: rail.height
            boundsBehavior: Flickable.StopAtBounds
            clip: true

            Column {
                id: rail
                width: 200
                spacing: 4

                RailTab { iconSource: "qrc:/qt/qml/org/archel/settings/src/gui/qml/images/home.svg";      label: "Accueil";       checked: true; onClicked: stack.currentIndex = 0 }
                RailTab { iconSource: "qrc:/qt/qml/org/archel/settings/src/gui/qml/images/network.svg";   label: "Réseau";         onClicked: stack.currentIndex = 1 }
                RailTab { iconSource: "qrc:/qt/qml/org/archel/settings/src/gui/qml/images/power.svg";     label: "Alimentation";   onClicked: stack.currentIndex = 2 }
                RailTab { iconSource: "qrc:/qt/qml/org/archel/settings/src/gui/qml/images/bluetooth.svg";  label: "Bluetooth";     onClicked: stack.currentIndex = 3 }
                RailTab { iconSource: "qrc:/qt/qml/org/archel/settings/src/gui/qml/images/appearance.svg"; label: "Apparence";     onClicked: stack.currentIndex = 4 }
                RailTab { iconSource: "qrc:/qt/qml/org/archel/settings/src/gui/qml/images/sound.svg";      label: "Son";            onClicked: stack.currentIndex = 5 }
                RailTab { iconSource: "qrc:/qt/qml/org/archel/settings/src/gui/qml/images/apps.svg";       label: "Applications";   onClicked: stack.currentIndex = 6 }
                RailTab { iconSource: "qrc:/qt/qml/org/archel/settings/src/gui/qml/images/display.svg";    label: "Affichage";      onClicked: stack.currentIndex = 7 }
                RailTab { iconSource: "qrc:/qt/qml/org/archel/settings/src/gui/qml/images/input.svg";      label: "Périphériques";  onClicked: stack.currentIndex = 8 }
                RailTab { iconSource: "qrc:/qt/qml/org/archel/settings/src/gui/qml/images/system.svg";     label: "Système";        onClicked: stack.currentIndex = 9 }
            }
        }

        // Tab content: one registered file per tab; hidden tabs are not
        // rendered (D-03). Landing (Accueil) is the default page (D-22).
        StackLayout {
            id: stack
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: 0

            LandingTab {}
            NetTab {
                id: netTab
                onImportRequested: vpnPopup.open()
            }
            PowerTab {}
            BtTab {}
            AppearanceTab { id: appearanceTab; theme: theme }
            AudioTab {}
            PkgTab { id: pkgTab }
            DisplayTab {}
            InputTab {}
            SystemTab {}
        }
    }

    // ── Global modal layer (06-06) ───────────────────────────────────
    // Visibility is driven solely by the session kind mirrored onto
    // ShellModel by the sync timer below — the modals never set their own
    // visibility, and each closes through an explicit ShellModel invokable
    // so the session and the view stay in lockstep (D-14/D-15).

    ConfirmModal {
        question: shell.modalQuestion
        visible: shell.modalKind === "confirm"
        onConfirmed: shell.confirmAccepted()
        onDeclined: shell.confirmDeclined()
        z: 100
    }

    // Stacked second confirm (D-06/D-08, plan 07-03): renders the top
    // stack entry above the base confirm; each resolves independently
    // through the top-resolving invokables, and cancelling the top
    // leaves older pending confirms stacked. Visibility derives only
    // from mirrored state (same NoAutoClose rule as the base modal).
    ConfirmModal {
        question: shell.modalQuestionTop
        visible: shell.modalKind === "confirm" && shell.modalConfirmCount > 1
        onConfirmed: shell.confirmTopAccepted()
        onDeclined: shell.confirmTopDeclined()
        z: 105
    }

    ErrorModal {
        errorText: shell.modalError
        visible: shell.modalKind === "error"
        onDismissed: shell.closeModal()
        z: 101
    }

    StreamModal {
        lines: shell.modalLines
        done: shell.modalDone
        operationResult: shell.modalResult
        trustNeeded: shell.modalTrustNeeded
        trustSummary: shell.modalTrustSummary
        pkgbuild: shell.modalPkgbuild
        pkgbuildSeen: shell.modalPkgbuildSeen
        trustConfirmed: shell.modalTrustConfirmed
        visible: shell.modalKind === "stream"
        // During an AUR trust review the stream already ended: Annuler must
        // dismiss the review (close), not signal a worker cancellation (D-19).
        onCancelRequested: shell.cancelStream()
        onDismissed: shell.closeModal()
        onPkgbuildSeenRequested: shell.notePkgbuildSeen()
        onTrustToggled: shell.confirmTrust(checked)
        // D-19: Valider is the only path to the build; the
        // authoritative gate re-checks both flags Rust-side.
        onAurValidated: pkgTab.pkg.aurBuildStart()
        z: 102
    }

    // VPN import popup (D-20, plan 06-07): single instance at the
    // root, opened by the NetTab Importer button. The chosen path
    // goes to the NetModel worker (validate-infer-confirm-import);
    // the popup closes on submit so the tab shows the spinner and
    // the result banner. Failures surface in the ErrorModal.
    VpnImportPopup {
        id: vpnPopup
        busy: netTab.net.vpnBusy
        onImportRequested: function (path) {
            netTab.net.vpnImportStart(path)
            close()
        }
        onDismissed: close()
        z: 103
    }

    // Bluetooth pairing modal (D-21, plan 06-07): the ONLY pairing
    // surface in the app, global by construction (it pops above
    // whichever tab is visible). Pure view over the ShellModel
    // pairing session: visibility follows pairing_active, the code
    // is the core-formatted passkey, and Confirmer/Refuser resolve
    // the waiting agent callback - never auto-accept.
    PairingModal {
        deviceName: shell.pairingDeviceName
        address: shell.pairingAddress
        passkey: shell.pairingPasskey
        visible: shell.pairingActive
        onConfirmed: shell.pairingAccepted()
        onDeclined: shell.pairingDeclined()
        z: 104
    }

    // Mirror ModalSession onto ShellModel's modal properties. 120 ms sits
    // well under the flicker threshold and coalesces a streaming flood into
    // at most one property write per tick.
    Timer {
        interval: 120
        repeat: true
        running: true
        onTriggered: shell.syncSession()
    }
}
