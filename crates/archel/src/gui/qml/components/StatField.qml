// StatField: label/value stat cell (UI-SPEC StatField contract).
//
// Empty value shows the pre-first-tick placeholder; the honest N/A token
// renders at 40% opacity, quiet, never red (N/A is not an error).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

RowLayout {
    property string fieldLabel: ""
    property string fieldValue: ""

    spacing: 8

    Label {
        text: fieldLabel
        font.pixelSize: Math.round(Qt.application.font.pixelSize * 0.85)
        Layout.preferredWidth: 140
        elide: Text.ElideRight
    }
    Label {
        text: fieldValue === "" ? "…" : fieldValue
        opacity: fieldValue === "N/A" ? 0.4 : 1.0
        Layout.fillWidth: true
        horizontalAlignment: Text.AlignRight
        elide: Text.ElideMiddle
    }
}
