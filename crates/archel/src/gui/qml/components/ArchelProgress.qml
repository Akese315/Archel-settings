// ArchelProgress: themed progress bar. Fusion's bar is a flat gray
// sunken panel; on the dark surfaces it reads as a smudge. This one
// draws an 8px rounded track (text-token veil) with an accent fill:
// proportional in determinate mode, an infinite accent sweep in
// indeterminate mode. Drop-in for ProgressBar: same properties.
import QtQuick
import QtQuick.Controls

ProgressBar {
    id: root

    opacity: enabled ? 1 : 0.5

    background: Rectangle {
        implicitWidth: 200
        implicitHeight: 8
        radius: 4
        color: Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.12)
    }

    contentItem: Item {
        id: track

        clip: true
        implicitWidth: root.background.implicitWidth
        implicitHeight: root.background.implicitHeight

        // Determinate fill, proportional to the value.
        Rectangle {
            visible: !root.indeterminate
            width: root.visualPosition * track.width
            height: track.height
            radius: 4
            color: palette.accent
        }

        // Indeterminate sweep (stream/worker phases with no total).
        Rectangle {
            id: sweep

            visible: root.indeterminate
            width: track.width * 0.3
            height: track.height
            radius: 4
            color: palette.accent

            SequentialAnimation on x {
                running: root.indeterminate && root.visible
                loops: Animation.Infinite
                NumberAnimation {
                    from: -sweep.width
                    to: track.width
                    duration: 1300
                    easing.type: Easing.InOutQuad
                }
            }
        }
    }
}
