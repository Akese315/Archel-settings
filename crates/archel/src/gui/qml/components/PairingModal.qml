// PairingModal (D-21): the single global Bluetooth pairing-code
// confirmation. Instantiated ONCE at the Main.qml root - no second
// pairing dialog may exist anywhere (grep PairingModal instantiation
// count == 1 in Main.qml).
//
// Pure view over the ShellModel pairing session (plan 06-07): it owns
// no state and never closes itself (NoAutoClose keeps the Main.qml
// `visible: shell.pairing_active` binding alive - a Dialog that closed
// imperatively would permanently break that binding). The worker's
// agent callback publishes the device name plus the zero-padded code
// (core format_passkey output, large monospace); Confirmer releases
// the waiter with true (the ONLY accept path - never auto-accept),
// Refuser with false (the worker then removes any half-bond and
// reports nothing). Frozen api: {deviceName, passkey, accepted,
// declined}.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    property string deviceName: ""
    property string address: ""
    property string passkey: ""
    signal accepted()
    signal declined()

    modal: true
    title: "Appariement Bluetooth"
    width: 420
    padding: 16
    closePolicy: Popup.NoAutoClose
    // Esc acts as Refuser (UI-SPEC keyboard row); the binding-driven
    // visibility never changes here, only the session decision does.
    onEscPressed: root.declined()

    contentHeight: col.implicitHeight

    ColumnLayout {
        id: col
        width: parent.width
        spacing: 16

        Label {
            Layout.fillWidth: true
            text: "Confirmez le code affiché sur « " + root.deviceName + " » (" + root.address + ") :"
            wrapMode: Label.WrapAnywhere
        }

        Label {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            text: root.passkey === "" ? "······" : root.passkey
            font.family: "monospace"
            font.pixelSize: Math.round(Qt.font({}).pixelSize * 2)
            font.bold: true
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8

            Button {
                text: "Refuser"
                Layout.minimumWidth: 96
                onClicked: root.declined()
            }
            Button {
                id: confirmButton
                text: "Confirmer"
                Layout.minimumWidth: 96
                highlighted: true
                enabled: root.passkey !== ""
                onClicked: root.accepted()
            }
        }
    }

    // Enter triggers the primary action (UI-SPEC keyboard contract).
    defaultButton: confirmButton
}
