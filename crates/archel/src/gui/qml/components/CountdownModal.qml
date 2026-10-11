// CountdownModal (D-16): the display keep-or-rollback countdown.
// Pure view over the DisplayModel countdown session (plan 06-08): it
// owns no state and never closes itself (NoAutoClose keeps the
// DisplayTab `visible: display.countdownVisible` binding alive - a
// Dialog that closed imperatively would permanently break that
// binding). DisplayTab wires keepRequested()/revertRequested() to
// display.keepChange()/revertChange(). The 15 s timeout itself lives
// in the worker's recv_timeout (core rolls back); this modal only
// displays the ticking second. Instantiated ONLY in DisplayTab.qml.
// Frozen api: {monitorName, secondsLeft, totalSeconds,
// keepRequested, revertRequested}.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    property string monitorName: ""
    property int secondsLeft: 0
    property int totalSeconds: 15
    signal keepRequested()
    signal revertRequested()

    modal: true
    title: "Affichage"
    width: 420
    padding: 16
    closePolicy: Popup.NoAutoClose
    // Esc acts as Annuler (UI-SPEC keyboard row); the binding-driven
    // visibility never changes here, only the session decision does.
    onEscPressed: root.revertRequested()

    contentHeight: col.implicitHeight

    ColumnLayout {
        id: col
        width: parent.width
        spacing: 16

        Label {
            Layout.fillWidth: true
            text: "Conserver ces paramètres ?"
            font.bold: true
            font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
        }

        Label {
            Layout.fillWidth: true
            text: monitorName
            elide: Text.ElideMiddle
        }

        Label {
            Layout.fillWidth: true
            text: "Annulation automatique dans " + secondsLeft + " s."
            font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.6)
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        ProgressBar {
            Layout.fillWidth: true
            from: 0
            to: totalSeconds
            value: secondsLeft
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8

            Button {
                text: "Annuler"
                Layout.minimumWidth: 96
                onClicked: root.revertRequested()
            }
            Button {
                id: keepButton
                text: "Conserver"
                Layout.minimumWidth: 96
                highlighted: true
                // Initial keyboard focus (UI-SPEC keyboard contract):
                // Dialog has no defaultButton property, the focused
                // button carries Enter/Space activation.
                focus: true
                onClicked: root.keepRequested()
            }
        }
    }
}
