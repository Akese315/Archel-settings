// PollBanner: dismissible D-05 poll-failure banner.
//
// The model shows it once per failure episode (bannerVisible); the close
// button asks the model to dismiss (stays silent until a success
// re-arms the policy). Copy is the locked UI-SPEC sentence.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Frame {
    id: root

    property bool bannerVisible: false
    signal dismissRequested()

    visible: bannerVisible
    Layout.fillWidth: true

    RowLayout {
        anchors.fill: parent
        spacing: 8

        Label {
            text: "Données indisponibles pour cette section."
            Layout.fillWidth: true
            wrapMode: Text.Wrap
        }
        Button {
            text: "Fermer"
            onClicked: root.dismissRequested()
        }
    }
}
