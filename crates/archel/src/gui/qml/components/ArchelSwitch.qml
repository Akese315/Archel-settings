// ArchelSwitch: themed toggle switch. Fusion's default is a small
// pale track that washes out on the dark surfaces. This one draws a
// 44x24 rounded track — accent-filled when on, text-token veil with
// a mid border when off — and a window-token knob that slides with a
// 150 ms ease, dark-on-accent in dark mode and light-on-accent in
// light mode (same token pairing as the ArchelButton label). Drop-in
// for Switch: same properties/signals.
import QtQuick
import QtQuick.Controls

Switch {
    id: root

    spacing: 8
    opacity: enabled ? 1 : 0.5

    indicator: Rectangle {
        implicitWidth: 44
        implicitHeight: 24
        x: root.leftPadding
        y: root.height / 2 - height / 2
        radius: 12

        color: root.checked
            ? palette.accent
            : Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.12)

        border.width: root.checked ? 0 : 1
        border.color: root.hovered
            ? Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.30)
            : palette.mid

        Rectangle {
            x: root.checked ? parent.width - width - 3 : 3
            y: parent.height / 2 - height / 2
            width: 18
            height: 18
            radius: 9
            color: root.checked
                ? palette.window
                : (root.hovered
                    ? Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.85)
                    : Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.65))

            Behavior on x {
                NumberAnimation { duration: 150; easing.type: Easing.OutCubic }
            }
            Behavior on color {
                ColorAnimation { duration: 150 }
            }
        }

        Behavior on color {
            ColorAnimation { duration: 150 }
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
