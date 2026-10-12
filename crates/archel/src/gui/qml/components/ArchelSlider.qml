// ArchelSlider: themed horizontal slider. Fusion's default track is
// a thin gray bar with a pale sunken handle that washes out on the
// dark surfaces. This one draws a 6px rounded track — text-token veil
// at rest with an accent fill up to the knob — and an 18px window-
// token knob (accent-bordered while pressed, accent halo on focus).
// Drop-in for Slider: same properties/signals.
import QtQuick
import QtQuick.Controls

Slider {
    id: root

    hoverEnabled: true
    opacity: enabled ? 1 : 0.5

    background: Rectangle {
        implicitWidth: 200
        implicitHeight: 18
        x: root.leftPadding
        y: root.topPadding + root.availableHeight / 2 - height / 2
        width: root.availableWidth
        height: 6
        radius: 3

        color: Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.12)

        Rectangle {
            width: root.visualPosition * parent.width
            height: parent.height
            radius: parent.radius
            color: palette.accent
        }
    }

    handle: Rectangle {
        implicitWidth: 18
        implicitHeight: 18
        radius: 9
        x: root.leftPadding + root.visualPosition * (root.availableWidth - width)
        y: root.topPadding + root.availableHeight / 2 - height / 2

        color: palette.window
        border.width: 1
        border.color: root.pressed
            ? palette.accent
            : (root.hovered
                ? Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.30)
                : palette.mid)

        Behavior on border.color {
            ColorAnimation { duration: 120 }
        }

        // Soft accent halo while focused, in the free margin ring.
        Rectangle {
            anchors.fill: parent
            anchors.margins: -2
            radius: 10
            color: "transparent"
            border.width: 2
            border.color: Qt.rgba(palette.accent.r, palette.accent.g, palette.accent.b, 0.35)
            visible: root.activeFocus && root.enabled
        }
    }
}
