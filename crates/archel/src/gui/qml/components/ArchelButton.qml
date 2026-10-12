// ArchelButton: themed push button. The default Fusion button paints
// with the translucent palette.button (white @6% in dark mode), which
// all but disappears on the dark surfaces — and Fusion's gradient
// chrome fights the catppuccin palette. This component derives the
// surface from the text token (same trick as ArchelField), so the
// button reads as a button in BOTH modes: rest/hover/press steps,
// accent fill + opaque window-token label on highlighted (primary)
// buttons, accent focus halo. Drop-in for Button: same properties and
// signals, only the chrome changes.
import QtQuick
import QtQuick.Controls

Button {
    id: root

    hoverEnabled: true
    opacity: enabled ? 1 : 0.5

    leftPadding: 16
    rightPadding: 16
    topPadding: 8
    bottomPadding: 8

    // Label: the opaque window token on the accent fill gives
    // dark-on-accent in dark mode and light-on-accent in light mode
    // (AA+ either way); the text token elsewhere.
    contentItem: Text {
        text: root.text
        font: root.font
        color: root.highlighted ? palette.window : palette.text
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        radius: 10

        // Surface steps from the text token — one notch above the
        // card at rest, two on hover, three pressed.
        color: root.highlighted
            ? palette.accent
            : Qt.rgba(palette.text.r,
                      palette.text.g,
                      palette.text.b,
                      root.pressed ? 0.16 : root.hovered ? 0.12 : 0.08)

        border.width: root.highlighted ? 0 : 1
        border.color: palette.mid

        // Pressed feedback on primary buttons: window-token veil
        // over the accent fill (no color math on opaque colors).
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            color: Qt.rgba(palette.window.r, palette.window.g, palette.window.b, 0.25)
            visible: root.highlighted && root.pressed
        }

        // Soft accent halo while focused, in the free margin ring.
        Rectangle {
            anchors.fill: parent
            anchors.margins: -2
            radius: parent.radius + 2
            color: "transparent"
            border.width: 2
            border.color: Qt.rgba(palette.accent.r, palette.accent.g, palette.accent.b, 0.35)
            visible: root.activeFocus && root.enabled
        }
    }
}
