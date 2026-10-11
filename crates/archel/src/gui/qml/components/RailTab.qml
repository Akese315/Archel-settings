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
    // Named iconSource, not icon: AbstractButton's `icon` is a FINAL
    // C++ grouped property and the engine refuses to load a QML
    // override of it (verified live on Qt 6.11.2).
    property string iconSource: ""
    property string label: ""
    // Not rendered (the custom contentItem draws the label) but it is
    // the button's accessible name for assistive tech.
    text: tab.label
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
            source: tab.iconSource
        }
        Label {
            text: tab.label
            Layout.fillWidth: true
            verticalAlignment: Text.AlignVCenter
            color: tab.checked ? tab.palette.highlightedText : tab.palette.windowText
        }
    }
}
