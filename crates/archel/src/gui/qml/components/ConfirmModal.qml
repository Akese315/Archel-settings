// ConfirmModal (D-12): every write opens this first - no exceptions.
// Pure view over the ShellModel modal session (plan 06-06): it owns no
// state and never closes itself (NoAutoClose keeps the Main.qml
// `visible: shell.modal_kind === "confirm"` binding alive - a Dialog that
// closed imperatively would permanently break that binding). Main.qml
// wires confirmed()/declined() to shell.confirmAccepted()/confirmDeclined().
// Fermer is a first-class decline (releases a waiting worker with false),
// never an error. `confirmed` (not `accepted`): Dialog already provides
// accepted(). Frozen api: {question, confirmed, declined}.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    property string question: ""
    signal confirmed()
    signal declined()

    modal: true
    title: "Archel"
    width: 420
    padding: 16
    closePolicy: Popup.NoAutoClose
    // Esc acts as Fermer (UI-SPEC keyboard row); the binding-driven
    // visibility never changes here, only the session decision does.
    // Popup/Dialog has no onEscPressed (and Keys only attaches to
    // Items, not to Popup) - a window-context Shortcut carries the
    // key instead; NoAutoClose still blocks any implicit close.
    Shortcut {
        enabled: root.visible
        sequence: "Escape"
        onActivated: root.declined()
    }

    contentHeight: col.implicitHeight

    ColumnLayout {
        id: col
        width: parent.width
        spacing: 16

        Label {
            Layout.fillWidth: true
            text: root.question
            wrapMode: Label.WrapAnywhere
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8

            Button {
                text: "Fermer"
                Layout.minimumWidth: 96
                onClicked: root.declined()
            }
            Button {
                id: confirmButton
                text: "Confirmer"
                Layout.minimumWidth: 96
                highlighted: true
                // Initial keyboard focus (UI-SPEC keyboard contract):
                // Dialog has no defaultButton property, the focused
                // button carries Enter/Space activation.
                focus: true
                onClicked: root.confirmed()
            }
        }
    }
}
