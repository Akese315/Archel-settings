// Alimentation tab placeholder (D-22 fixed tab set). Registered in build.rs now so
// later GUI plans fill content without touching the qrc list (Pitfall 2).
import QtQuick
import QtQuick.Controls

Item {
    Label {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.margins: 16
        text: "Alimentation"
        font.bold: true
        font.pixelSize: 20
    }
}
