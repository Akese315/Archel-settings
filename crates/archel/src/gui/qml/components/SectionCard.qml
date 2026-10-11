// SectionCard: rounded, palette-driven section container (spec
// 2026-10-11). Drop-in replacement for the default GroupBox styling:
// same `title` property and content layout, so the tab-wide
// substitution in later tasks is mechanical.
import QtQuick
import QtQuick.Controls

GroupBox {
    id: root

    background: Rectangle {
        color: root.palette.base
        radius: 12
        border.color: root.palette.mid
        border.width: 1
    }

    label: Label {
        text: root.title
        color: root.palette.windowText
        font.bold: true
        font.pixelSize: Math.round(Qt.application.font.pixelSize * 0.95)
        opacity: 0.85
        x: root.leftPadding
        width: root.availableWidth
        elide: Text.ElideRight
        y: Math.round((root.topPadding - implicitHeight) / 2)
    }
}
