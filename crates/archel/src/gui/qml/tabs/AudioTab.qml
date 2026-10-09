// Son tab: sink/source lists with default markers (read-only).
//
// The Timer runs unconditionally so the model keeps ticking while this
// tab is hidden (D-03/D-06). Failures read the empty lists plus the
// once-per-episode banner (D-04/D-05). Device names elide in the middle
// with the full text in a ToolTip (UI-SPEC long-text backstop).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    AudioModel {
        id: audio
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: audio.tick()
    }
    Component.onCompleted: audio.tick()

    ScrollView {
        id: view
        anchors.fill: parent
        anchors.margins: 16
        clip: true
        contentWidth: availableWidth

        ColumnLayout {
            width: view.availableWidth
            spacing: 24

            Label {
                text: "Son"
                font.bold: true
                font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: audio.bannerVisible
                onDismissRequested: audio.dismissBanner()
            }

            GroupBox {
                title: "Lecture"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    ListView {
                        id: sinkList
                        Layout.fillWidth: true
                        Layout.preferredHeight: contentHeight
                        interactive: false
                        clip: true
                        model: audio.sinksJson === "" ? [] : JSON.parse(audio.sinksJson)
                        delegate: ItemDelegate {
                            width: sinkList.width
                            hoverEnabled: true
                            ToolTip.visible: hovered
                            ToolTip.text: modelData.name
                            contentItem: RowLayout {
                                spacing: 8
                                Label {
                                    text: modelData.default ? "●" : "○"
                                    font.pixelSize: Math.round(Qt.font({}).pixelSize * 0.85)
                                }
                                Label {
                                    text: modelData.description
                                    Layout.fillWidth: true
                                    elide: Text.ElideMiddle
                                }
                            }
                        }
                    }
                }
            }

            GroupBox {
                title: "Enregistrement"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    ListView {
                        id: sourceList
                        Layout.fillWidth: true
                        Layout.preferredHeight: contentHeight
                        interactive: false
                        clip: true
                        model: audio.sourcesJson === "" ? [] : JSON.parse(audio.sourcesJson)
                        delegate: ItemDelegate {
                            width: sourceList.width
                            hoverEnabled: true
                            ToolTip.visible: hovered
                            ToolTip.text: modelData.name
                            contentItem: RowLayout {
                                spacing: 8
                                Label {
                                    text: modelData.default ? "●" : "○"
                                    font.pixelSize: Math.round(Qt.font({}).pixelSize * 0.85)
                                }
                                Label {
                                    text: modelData.description
                                    Layout.fillWidth: true
                                    elide: Text.ElideMiddle
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
