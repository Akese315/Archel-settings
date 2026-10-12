// ArchelCheck: themed checkbox. Same chrome language as ArchelRadio
// (text-token surfaces, mid border, accent fill) but squared: rounded
// 6px box that fills with the accent and draws a window-token check
// when ticked. Drop-in for CheckBox: same properties/signals.
import QtQuick
import QtQuick.Controls

CheckBox {
    id: root

    spacing: 8
    opacity: enabled ? 1 : 0.5

    indicator: Rectangle {
        implicitWidth: 20
        implicitHeight: 20
        x: root.leftPadding
        y: root.height / 2 - height / 2
        radius: 6

        color: root.checked ? palette.accent : "transparent"

        border.width: 1
        border.color: root.checked
            ? palette.accent
            : (root.hovered
                ? Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.30)
                : palette.mid)

        Text {
            anchors.centerIn: parent
            text: "\u2713"
            font.pixelSize: 13
            font.bold: true
            color: palette.window
            visible: root.checked
            opacity: root.checked ? 1 : 0

            Behavior on opacity {
                NumberAnimation { duration: 120 }
            }
        }

        Behavior on color {
            ColorAnimation { duration: 120 }
        }
    }

    contentItem: Text {
        text: root.text
        font: root.font
        color: palette.text
        opacity: root.enabled ? 1 : 0.6
        leftPadding: root.indicator.width + root.spacing
        verticalAlignment: Text.AlignVCenter
    }
}
