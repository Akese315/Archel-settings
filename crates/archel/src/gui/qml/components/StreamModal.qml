// StreamModal (D-13/D-14): the one privileged-operation view. Terminal-
// like by monospace + palette.base only - never a hardcoded dark theme
// (D-24 system-follow). Pure view over the ShellModel modal session
// (plan 06-06): lines arrive via Mutex + Timer-poll into `lines`; the
// footer is Annuler while running and Fermer once done. D-15: while
// !done the only enabled control is Annuler and Esc is blocked - an
// interruption is an explicit decision, never a disappearance. The end
// state is data, not an error (D-14): result carries the ASCII key
// ("completed"/"cancelled"/"failed") and the French sentence lives here.
// The trust block (D-19 summary card + PKGBUILD expander + trust
// checkbox + Valider) is added by Task 3 of the same plan on these same
// frozen properties. Frozen api: {lines, done, result, trustNeeded,
// trustSummary, pkgbuild, pkgbuildSeen, trustConfirmed, cancelRequested,
// closed, aurValidated, pkgbuildSeenRequested, trustToggled}.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    property string lines: ""
    property bool done: false
    property string result: ""
    // D-19 trust-gate surface (rendered by Task 3; carried now so the
    // api block stays frozen for 06-07/08).
    property bool trustNeeded: false
    property string trustSummary: ""
    property string pkgbuild: ""
    property bool pkgbuildSeen: false
    property bool trustConfirmed: false

    signal cancelRequested()
    signal closed()
    signal aurValidated()
    signal pkgbuildSeenRequested()
    signal trustToggled(bool checked)

    modal: true
    title: "Opération en cours"
    width: 560
    padding: 16
    closePolicy: Popup.NoAutoClose
    // D-15: Esc cannot dismiss a running operation; once done it acts
    // as Fermer.
    onEscPressed: {
        if (root.done) {
            root.closed();
        }
    }

    SystemPalette {
        id: pal
    }

    contentHeight: col.implicitHeight

    ColumnLayout {
        id: col
        width: parent.width
        spacing: 16

        ScrollView {
            id: logView
            Layout.fillWidth: true
            Layout.preferredHeight: 240
            clip: true
            background: Rectangle {
                color: pal.base
            }

            TextEdit {
                id: logText
                readOnly: true
                text: root.lines
                wrapMode: TextEdit.WrapAnywhere
                font.family: "monospace"
                font.pointSize: 13
                color: pal.text
                selectionColor: pal.highlight
                // Auto-scroll to the newest line (UI-SPEC streaming-log
                // overflow row); the buffer is capped core-side, this is
                // just the viewport follow.
                width: logView.availableWidth > 0 ? logView.availableWidth : root.width - 2 * root.padding
                onTextChanged: logView.contentItem.contentY = Math.max(
                    0, logView.contentItem.contentHeight - logView.height)
            }
        }

        ProgressBar {
            Layout.fillWidth: true
            visible: !root.done
            indeterminate: true
        }

        // D-14: the end of the transaction is reported, never collapsed
        // into an error - a cancelled stop reads as cancelled.
        Label {
            Layout.fillWidth: true
            visible: root.done
            text: root.result === "completed" ? "Opération terminée."
                  : root.result === "cancelled" ? "Opération annulée (interrompue par l’utilisateur)."
                  : "Opération échouée."
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8

            Button {
                id: footerButton
                Layout.minimumWidth: 96
                highlighted: true
                text: root.done ? "Fermer" : "Annuler"
                // One footer control that swaps roles (D-15: Annuler is
                // the only enabled button while running). Annuler only
                // requests the stop; the modal stays until the worker
                // reports the end state.
                onClicked: root.done ? root.closed() : root.cancelRequested()
            }
        }
    }

    defaultButton: footerButton
}
