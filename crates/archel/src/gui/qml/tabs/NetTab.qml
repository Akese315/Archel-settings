// Réseau tab: live connectivity state plus the VPN list (read-only).
//
// The Timer runs unconditionally so the model keeps ticking while this
// tab is hidden (D-03/D-06). Pre-first-tick values read the ellipsis
// token; failures read N/A plus the once-per-episode banner (D-04/D-05).
// Device names elide in the middle with the full text in a ToolTip
// (UI-SPEC long-text backstop). The Importer button is an affordance
// only: plan 07 wires it to the VpnImportPopup (D-20).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    NetModel {
        id: net
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: net.tick()
    }
    Component.onCompleted: net.tick()

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
                text: "Réseau"
                font.bold: true
                font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: net.bannerVisible
                onDismissRequested: net.dismissBanner()
            }

            GroupBox {
                title: "Connexion"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "État"
                        fieldValue: net.statusText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Interface"
                        fieldValue: net.ifaceText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "SSID"
                        fieldValue: net.ssidText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "IPv4"
                        fieldValue: net.ipv4Text
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Passerelle"
                        fieldValue: net.gatewayText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "DNS"
                        fieldValue: net.dnsText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Profil"
                        fieldValue: net.profileText
                    }
                }
            }

            GroupBox {
                title: "VPN"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    ListView {
                        id: vpnList
                        Layout.fillWidth: true
                        Layout.preferredHeight: contentHeight
                        interactive: false
                        clip: true
                        model: net.vpnJson === "" ? [] : JSON.parse(net.vpnJson)
                        delegate: ItemDelegate {
                            width: vpnList.width
                            hoverEnabled: true
                            ToolTip.visible: hovered
                            ToolTip.text: modelData.name
                            contentItem: RowLayout {
                                spacing: 8
                                Label {
                                    text: modelData.name
                                    Layout.fillWidth: true
                                    elide: Text.ElideMiddle
                                }
                                Label {
                                    text: modelData.vpn_type
                                    font.pixelSize: Math.round(Qt.font({}).pixelSize * 0.85)
                                    opacity: 0.6
                                }
                                Label {
                                    text: modelData.active ? "Actif" : ""
                                    font.pixelSize: Math.round(Qt.font({}).pixelSize * 0.85)
                                }
                            }
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        visible: net.vpnJson !== "" && JSON.parse(net.vpnJson).length === 0

                        Label {
                            text: "Aucun VPN configuré"
                            font.bold: true
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }
                        Label {
                            text: "Importez une configuration OpenVPN ou WireGuard avec le bouton « Importer »."
                            wrapMode: Text.Wrap
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }

                    Button {
                        text: "Importer"
                        // Affordance only (D-20): plan 07 wires this to the VpnImportPopup.
                        onClicked: {}
                    }
                }
            }
        }
    }
}
