// PageHeader: tab page title + optional subtitle (spec 2026-10-11).
// Control root (transparent background, zero padding) so the Labels
// inherit the window palette; callers put it in a ColumnLayout and set
// Layout.fillWidth themselves.
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

Control {
    id: root

    property string pageTitle: ""
    property string pageSubtitle: ""

    padding: 0
    background: null

    implicitHeight: headerCol.implicitHeight

    contentItem: ColumnLayout {
        id: headerCol
        spacing: 2

        Label {
            text: root.pageTitle
            font.bold: true
            font.pixelSize: Math.round(Qt.application.font.pixelSize * 1.3)
            color: root.palette.windowText
        }
        Label {
            text: root.pageSubtitle
            visible: root.pageSubtitle !== ""
            font.pixelSize: Math.round(Qt.application.font.pixelSize * 0.9)
            color: root.palette.placeholderText
        }
    }
}
