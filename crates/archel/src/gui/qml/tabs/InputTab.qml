// Périphériques tab: mouse leaves plus the merged keybind inventory
// (read-only; the Ajouter editor lands in plan 08).
//
// The Timer runs unconditionally so the model keeps ticking while this
// tab is hidden (D-03/D-06). Pre-first-tick values read the ellipsis
// token; failures read N/A plus the once-per-episode banner (D-04/D-05).
// Keybind rows render the core's render strings verbatim via the
// combo/detail pairs; long summaries elide in the middle with the full
// text in a ToolTip (UI-SPEC long-text backstop). The Ajouter button is
// an affordance only: plan 08 wires it to the keybind editor (D-12).
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
                            enabled: false
                        }
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Valeur"
                        fieldValue: input.sensitivityText
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

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Facteur de défilement"
                        fieldValue: input.scrollText
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

                    Button {
                        text: "Ajouter"
                        // Affordance only: plan 08 wires this to the keybind editor.
                        onClicked: {}
                    }
                }
            }
        }
    }
}
