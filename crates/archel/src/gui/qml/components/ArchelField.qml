// ArchelField: themed single-line input. The default Fusion TextField
// paints with the translucent palette.base, which all but disappears on
// the near-black dark window (spec 2026-10-11 dark base is white @5%).
// This component derives an opaque-enough surface from the text token
// so the field reads as an input in BOTH modes, with the accent ring
// on focus and a hover-visible border. Drop-in for TextField: same
// properties/signals, only the chrome changes.
import QtQuick
import QtQuick.Controls

TextField {
    id: root

    selectByMouse: true
    hoverEnabled: true
    opacity: enabled ? 1 : 0.5

    color: palette.text
    placeholderTextColor: palette.placeholderText
    selectionColor: Qt.rgba(palette.accent.r, palette.accent.g, palette.accent.b, 0.35)
    selectedTextColor: palette.text

    leftPadding: 12
    rightPadding: 12
    topPadding: 8
    bottomPadding: 8

    background: Rectangle {
        radius: 10

        // Surface: the text token at low alpha — white-ish over the
        // dark surfaces, gray over the light card — guaranteed to sit
        // one step above the surrounding card in both modes.
        color: Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.08)

        border.width: 1
        border.color: root.activeFocus
            ? palette.accent
            : (root.hovered
                ? Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.30)
                : palette.mid)

        // Soft accent halo while focused: a 2px accent outline with
        // 35% alpha, drawn in the 2px margins the border leaves free.
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
