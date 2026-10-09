// ConfirmModal (D-12): every write opens this first - no exceptions.
// Pure view over the ShellModel modal session (plan 06-06): it owns no
// state and never closes itself (NoAutoClose keeps the Main.qml
// `visible: shell.modal_kind === "confirm"` binding alive - a Dialog that
// closed imperatively would permanently break that binding). Main.qml
// wires accepted()/declined() to shell.confirmAccepted()/confirmDeclined().
// Fermer is a first-class decline (releases a waiting worker with false),
// never an error. Frozen api: {question, accepted, declined}.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    property string question: ""
    signal accepted()
    signal declined()

    modal: true
    title: "Archel"
    width: 420
    padding: 16
    closePolicy: Popup.NoAutoClose
    // Esc acts as Fermer (UI-SPEC keyboard row); the binding-driven
    // visibility never changes here, only the session decision does.
    onEscPressed: root.declined()

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
                onClicked: root.accepted()
            }
        }
    }

    // Enter triggers the primary action (UI-SPEC keyboard contract).
    defaultButton: confirmButton
}
