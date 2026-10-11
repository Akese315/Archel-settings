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
    x: shell.launch_x
    y: shell.launch_y
    width: shell.launch_width
    height: shell.launch_height
    minimumWidth: 720
    minimumHeight: 480
    title: "Archel Réglages"
    visibility: shell.launch_maximized ? Window.Maximized : Window.Auto

    // Geometry bridge (bridge.rs). Defaults mirror the retired QML
    // literals, so the window is correct even before validation runs.
    ShellModel {
        id: shell
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

                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    checked: true
                    onClicked: stack.currentIndex = 0
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: "images/home.svg"
                        }
                        Label {
                            text: "Accueil"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 1
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: "images/network.svg"
                        }
                        Label {
                            text: "Réseau"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 2
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: "images/power.svg"
                        }
                        Label {
                            text: "Alimentation"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 3
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: "images/bluetooth.svg"
                        }
                        Label {
                            text: "Bluetooth"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 4
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: "images/appearance.svg"
                        }
                        Label {
                            text: "Apparence"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 5
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: "images/sound.svg"
                        }
                        Label {
                            text: "Son"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 6
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: "images/apps.svg"
                        }
                        Label {
                            text: "Applications"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 7
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: "images/display.svg"
                        }
                        Label {
                            text: "Affichage"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 8
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: "images/input.svg"
                        }
                        Label {
                            text: "Périphériques"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 9
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: "images/system.svg"
                        }
                        Label {
                            text: "Système"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
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
            AppearanceTab {}
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
        question: shell.modal_question
        visible: shell.modal_kind === "confirm"
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
        errorText: shell.modal_error
        visible: shell.modal_kind === "error"
        onDismissed: shell.closeModal()
        z: 101
    }

    StreamModal {
        lines: shell.modal_lines
        done: shell.modal_done
        operationResult: shell.modal_result
        trustNeeded: shell.modal_trust_needed
        trustSummary: shell.modal_trust_summary
        pkgbuild: shell.modal_pkgbuild
        pkgbuildSeen: shell.modal_pkgbuild_seen
        trustConfirmed: shell.modal_trust_confirmed
        visible: shell.modal_kind === "stream"
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
        deviceName: shell.pairing_device_name
        address: shell.pairing_address
        passkey: shell.pairing_passkey
        visible: shell.pairing_active
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
