// ArchelRadio: themed radio button. The Fusion indicator ignores the
// accent palette in dark mode and blends into the card; this one draws
// a 20px ring that fills with the accent when checked (animated dot)
// and keeps the label in the window text token. Drop-in for
// RadioButton: same properties/signals.
import QtQuick
import QtQuick.Controls

RadioButton {
    id: root

    spacing: 8
    opacity: enabled ? 1 : 0.5

    indicator: Rectangle {
        implicitWidth: 20
        implicitHeight: 20
        x: root.leftPadding
        y: root.height / 2 - height / 2
        radius: 10
        color: "transparent"

        border.width: root.checked ? 2 : 1
        border.color: root.checked
            ? palette.accent
            : (root.hovered
                ? Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.30)
                : palette.mid)

        Rectangle {
            anchors.centerIn: parent
            width: 10
            height: 10
            radius: 5
            color: palette.accent
            scale: root.checked ? 1 : 0
            visible: root.checked

            Behavior on scale {
                NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
            }
        }

        Behavior on border.color {
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
