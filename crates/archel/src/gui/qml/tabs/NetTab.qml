// Réseau tab: live connectivity state, DNS editor and VPN list with
// the plan-07 writes behind the D-12 confirm modal.
//
// The Timer runs unconditionally so the model keeps ticking while this
// tab is hidden (D-03/D-06). Pre-first-tick values read the ellipsis
// token; failures read N/A plus the once-per-episode banner (D-04/D-05).
//
// DNS rows edit locally and validate live through the core validator
// (French inline errors, same gate the apply worker re-checks before
// planning - T-06-31); Appliquer plans against the active profile,
// confirms with the plan's own question verbatim, then applies with a
// spinner on the button (D-15). The VPN Importer button opens the
// global VpnImportPopup (Main.qml root, D-20) via importRequested;
// the import result banner lands here, failures in the ErrorModal.
// Device names elide in the middle with the full text in a ToolTip
// (UI-SPEC long-text backstop).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    id: root

    property alias net: net
    signal importRequested()

    // Local DNS editor state: refreshed explicitly (ListModel row
    // edits emit no count change, so no binding can track them).
    property string dnsError: ""
    property bool dnsValid: false

    NetModel {
        id: net
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: net.tick()
    }
    Component.onCompleted: {
        net.tick()
        syncDnsFromCurrent()
    }

    ListModel {
        id: dnsRows
    }

    // Non-blank trimmed servers from the editor rows.
    function dnsArray() {
        var out = []
        for (var i = 0; i < dnsRows.count; i++) {
            var server = (dnsRows.get(i).server || "").trim()
            if (server !== "") {
                out.push(server)
            }
        }
        return out
    }

    // Re-runs the core validator over the editor rows; drives the
    // inline error and the Appliquer enablement.
    function refreshDnsState() {
        var servers = dnsArray()
        var err = net.dnsValidate(JSON.stringify(servers))
        dnsError = err
        dnsValid = err === "" && servers.length > 0
    }

    // Copies the live DNS row back into the editor (D-04 honest N/A
    // becomes a single empty row, never a guessed address).
    function syncDnsFromCurrent() {
        dnsRows.clear()
        var parts = net.dnsText.split(",")
        var added = 0
        for (var i = 0; i < parts.length; i++) {
            var server = parts[i].trim()
            if (server !== "" && server !== "N/A") {
                dnsRows.append({ server: server })
                added++
            }
        }
        if (added === 0) {
            dnsRows.append({ server: "" })
        }
        refreshDnsState()
    }

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
                title: "Serveurs DNS"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    Repeater {
                        model: dnsRows
                        delegate: RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            TextField {
                                Layout.fillWidth: true
                                enabled: !net.dnsBusy
                                text: model.server
                                placeholderText: "1.1.1.1"
                                onTextEdited: {
                                    dnsRows.setProperty(index, "server", text)
                                    refreshDnsState()
                                }
                            }
                            Button {
                                text: "Retirer"
                                enabled: !net.dnsBusy && dnsRows.count > 1
                                onClicked: {
                                    dnsRows.remove(index)
                                    refreshDnsState()
                                }
                            }
                        }
                    }

                    Label {
                        Layout.fillWidth: true
                        visible: dnsError !== ""
                        text: dnsError
                        wrapMode: Label.WrapAnywhere
                    }

                    RowLayout {
                        spacing: 8

                        Button {
                            text: "Ajouter"
                            enabled: !net.dnsBusy
                            onClicked: {
                                dnsRows.append({ server: "" })
                                refreshDnsState()
                            }
                        }
                        Button {
                            text: "Réinitialiser"
                            enabled: !net.dnsBusy
                            onClicked: syncDnsFromCurrent()
                        }
                        Button {
                            text: "Appliquer"
                            highlighted: true
                            enabled: dnsValid && !net.dnsBusy
                            onClicked: net.dnsApply(JSON.stringify(dnsArray()))
                        }
                        BusyIndicator {
                            visible: net.dnsBusy
                            running: net.dnsBusy
                        }
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

                    RowLayout {
                        spacing: 8

                        Button {
                            text: "Importer…"
                            enabled: !net.vpnBusy
                            onClicked: root.importRequested()
                        }
                        BusyIndicator {
                            visible: net.vpnBusy
                            running: net.vpnBusy
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        visible: net.vpnResultText !== ""

                        Label {
                            text: "VPN importé : " + net.vpnResultText
                            Layout.fillWidth: true
                            wrapMode: Label.WrapAnywhere
                        }
                        Button {
                            text: "Fermer"
                            onClicked: net.dismissVpnResult()
                        }
                    }
                }
            }
        }
    }
}
