// ArchelComboBox: themed combo box. The default Fusion combo paints
// with the translucent palette.base and a white popup — both wash out
// on the dark surfaces. This component reuses the ArchelField chrome
// (text-token surface, mid border, accent on open/focus, halo) and an
// opaque popup (window + text veil) with accent-highlighted rows, so
// the selector is readable in BOTH modes. Drop-in for ComboBox: same
// properties/signals, only the chrome changes.
import QtQuick
import QtQuick.Controls

ComboBox {
    id: root

    hoverEnabled: true
    opacity: enabled ? 1 : 0.5

    leftPadding: 12
    rightPadding: 12
    topPadding: 8
    bottomPadding: 8

    contentItem: Text {
        leftPadding: 0
        rightPadding: root.indicator.width + root.spacing
        text: root.displayText
        font: root.font
        color: palette.text
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    indicator: Text {
        x: root.width - width - 10
        y: root.height / 2 - height / 2
        text: "\u25be"
        font.pixelSize: Math.round(root.font.pixelSize * 0.9)
        color: root.popup.visible ? palette.accent : palette.placeholderText
    }

    background: Rectangle {
        radius: 10

        color: Qt.rgba(palette.text.r,
                      palette.text.g,
                      palette.text.b,
                      root.popup.visible ? 0.12 : root.hovered ? 0.12 : 0.08)

        border.width: 1
        border.color: root.popup.visible || root.activeFocus
            ? palette.accent
            : (root.hovered
                ? Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.30)
                : palette.mid)

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

    // Opaque elevated panel: the (opaque) window color under a text-
    // token veil, so nothing shows through in dark mode.
    popup: Popup {
        y: root.height + 4
        width: root.width
        padding: 6

        implicitHeight: Math.min(contentItem.implicitHeight + 2 * padding, 320)

        background: Rectangle {
            radius: 10
            color: palette.window
            border.width: 1
            border.color: palette.mid

            Rectangle {
                anchors.fill: parent
                radius: parent.radius
                color: Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.08)
            }
        }

        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight
            model: root.popup.visible ? root.delegateModel : null
            currentIndex: root.highlightedIndex
            spacing: 2
        }
    }

    delegate: ItemDelegate {
        id: comboItem

        required property var modelData
        required property int index

        width: ListView.view.width
        highlighted: root.highlightedIndex === comboItem.index

        // textRole lookup over object rows, plain modelData
        // otherwise (string arrays).
        text: comboItem.modelData !== undefined
              && root.textRole !== ""
              && comboItem.modelData[root.textRole] !== undefined
            ? String(comboItem.modelData[root.textRole])
            : String(comboItem.modelData)

        contentItem: Text {
            leftPadding: 8
            rightPadding: 8
            text: comboItem.text
            font: root.font
            color: comboItem.highlighted ? palette.window : palette.text
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }

        background: Rectangle {
            radius: 6
            color: comboItem.highlighted
                ? palette.accent
                : comboItem.hovered
                    ? Qt.rgba(palette.text.r, palette.text.g, palette.text.b, 0.12)
                    : "transparent"
        }
    }
}
