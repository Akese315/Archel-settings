// Périphériques tab: mouse leaves plus the managed keybind inventory
// (writable, plan 06-08).
//
// Pointer writes and keybind add/remove confirm first in the global
// ConfirmModal (D-12); keybind typos render inline under the editor
// (never a modal). Managed rows carry their exact-match address and
// show a red Supprimer button; compositor-reported (user) binds are
// display-only (D-06). Long combos/actions elide with the full text
// in a ToolTip (UI-SPEC long-text backstop).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    InputModel {
        id: input
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: input.tick()
    }
    Component.onCompleted: input.tick()

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
                text: "Périphériques"
                font.bold: true
                font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: input.bannerVisible
                onDismissRequested: input.dismissBanner()
            }

            BusyIndicator {
                running: input.operationBusy
                visible: input.operationBusy
                Layout.alignment: Qt.AlignHCenter
            }

            GroupBox {
                title: "Souris"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Label {
                            text: "Sensibilité"
                            Layout.preferredWidth: 140
                        }
                        Slider {
                            id: sensitivitySlider
                            Layout.fillWidth: true
                            from: -1
                            to: 1
                            stepSize: 0.01
                            value: {
                                if (input.sensitivityText === "")
                                    return 0;
                                var v = parseFloat(input.sensitivityText);
                                return isNaN(v) ? 0 : v;
                            }
                            enabled: !input.operationBusy
                        }
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Valeur"
                        fieldValue: input.sensitivityText
                    }
                    Button {
                        text: "Appliquer"
                        enabled: !input.operationBusy
                        onClicked: input.setSensitivity(sensitivitySlider.value.toFixed(2))
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Label {
                            text: "Défilement naturel"
                            Layout.fillWidth: true
                        }
                        Switch {
                            checked: input.naturalText === "on"
                            enabled: false
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Label {
                            text: "Accélération"
                            Layout.preferredWidth: 140
                        }
                        TextField {
                            id: accelerateField
                            Layout.fillWidth: true
                            placeholderText: "Facteur (ex. 1.50)"
                            text: input.scrollText === "N/A" ? "" : input.scrollText
                            enabled: !input.operationBusy
                        }
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Facteur de défilement"
                        fieldValue: input.scrollText
                    }
                    Button {
                        text: "Appliquer"
                        enabled: !input.operationBusy && accelerateField.text !== ""
                        onClicked: input.setAccelerate(accelerateField.text)
                    }
                }
            }

            GroupBox {
                title: "Raccourcis clavier"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    ListView {
                        id: bindsList
                        Layout.fillWidth: true
                        Layout.preferredHeight: contentHeight
                        interactive: false
                        clip: true
                        model: input.bindsJson === "" ? [] : JSON.parse(input.bindsJson)
                        delegate: ItemDelegate {
                            width: bindsList.width
                            hoverEnabled: true
                            ToolTip.visible: hovered
                            ToolTip.text: modelData.combo + " — " + modelData.detail
                            contentItem: RowLayout {
                                spacing: 8
                                Label {
                                    text: modelData.combo
                                    Layout.fillWidth: true
                                    elide: Text.ElideMiddle
                                }
                                Label {
                                    text: modelData.detail
                                    font.pixelSize: Math.round(Qt.font({}).pixelSize * 0.85)
                                    opacity: 0.6
                                    elide: Text.ElideMiddle
                                    Layout.maximumWidth: parent.width * 0.4
                                }
                                Button {
                                    text: "Supprimer"
                                    visible: modelData.managed === true
                                    enabled: !input.operationBusy
                                    contentItem: Label {
                                        text: parent.text
                                        color: "#c01c28"
                                        horizontalAlignment: Text.AlignHCenter
                                        verticalAlignment: Text.AlignVCenter
                                    }
                                    onClicked: input.removeKeybind(modelData.mods, modelData.key, modelData.dispatcher, modelData.arg)
                                }
                            }
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        visible: input.bindsJson !== "" && JSON.parse(input.bindsJson).length === 0

                        Label {
                            text: "Aucun raccourci personnalisé"
                            font.bold: true
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }
                        Label {
                            text: "Ajoutez votre premier raccourci avec « Ajouter »."
                            wrapMode: Text.Wrap
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        TextField {
                            id: comboField
                            Layout.fillWidth: true
                            placeholderText: "Combinaison (ex. SUPER + T)"
                            enabled: !input.operationBusy
                        }
                        TextField {
                            id: actionField
                            Layout.fillWidth: true
                            placeholderText: "Action (ex. exec kitty)"
                            enabled: !input.operationBusy
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        text: input.keybindError
                        visible: input.keybindError !== ""
                        wrapMode: Text.Wrap
                        color: "#c01c28"
                    }
                    Button {
                        text: "Ajouter"
                        enabled: !input.operationBusy && comboField.text !== "" && actionField.text !== ""
                        onClicked: input.addKeybind(comboField.text, actionField.text)
                    }
                }
            }
        }
    }
}
