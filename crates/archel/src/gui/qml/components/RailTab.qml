// RailTab: one restyled vertical-rail tab (spec 2026-10-11): icon +
// label, 36px tall, rounded 8px, active tab tinted with the accent,
// hover using the elevated surface. The click handler stays at the
// call site (Main.qml, where `stack` is lexically visible) because a
// separate component file cannot see Main.qml's ids. Colors come
// only from the tab's own inherited palette — same reason, no id
// references.
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

TabButton {
    id: tab
    property string icon: ""
    property string label: ""
    width: parent.width
    height: 36
    checkable: true
    autoExclusive: true
    background: Rectangle {
        radius: 8
        color: tab.checked ? tab.palette.highlight
             : tab.hovered ? tab.palette.alternateBase
             : "transparent"
    }
    contentItem: RowLayout {
        spacing: 6
        Image {
            Layout.preferredWidth: 16
            Layout.preferredHeight: 16
            sourceSize.width: 16
            sourceSize.height: 16
            source: tab.icon
        }
        Label {
            text: tab.label
            Layout.fillWidth: true
            verticalAlignment: Text.AlignVCenter
            color: tab.checked ? tab.palette.highlightedText : tab.palette.windowText
        }
    }
}
