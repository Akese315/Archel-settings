// Applications tab: installed packages, live search and package card
// (read-only; Installer/Retirer/Mettre à jour land in plan 06).
//
// The Timer runs unconditionally so the model keeps ticking while this
// tab is hidden (D-03/D-06). Pre-first-tick values read the ellipsis
// token; failures read N/A plus the once-per-episode banner (D-04/D-05).
// The search field calls search() on accepted text; rapid typing never
// flashes stale hits (per-query sequence tag, T-06-26). A zero-hit
// search renders the refinement copy; selecting a row loads the card
// through the core's label set. Names elide in the middle with the full
// text in a ToolTip (UI-SPEC long-text backstop).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    PkgModel {
        id: pkg
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: pkg.tick()
    }
    Component.onCompleted: pkg.tick()

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
                text: "Applications"
                font.bold: true
                font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.25)
            }

            PollBanner {
                bannerVisible: pkg.bannerVisible
                onDismissRequested: pkg.dismissBanner()
            }

            GroupBox {
                title: "Paquets"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    TextField {
                        id: searchField
                        Layout.fillWidth: true
                        placeholderText: "Rechercher un paquet…"
                        onAccepted: pkg.search(text)
                    }

                    BusyIndicator {
                        running: pkg.searchBusy
                        visible: pkg.searchBusy
                        Layout.alignment: Qt.AlignHCenter
                    }

                    ListView {
                        id: pkgList
                        Layout.fillWidth: true
                        Layout.preferredHeight: contentHeight
                        interactive: false
                        clip: true
                        model: {
                            if (pkg.searchJson !== "")
                                return JSON.parse(pkg.searchJson);
                            if (pkg.installedJson === "")
                                return [];
                            return JSON.parse(pkg.installedJson);
                        }
                        delegate: ItemDelegate {
                            width: pkgList.width
                            hoverEnabled: true
                            ToolTip.visible: hovered
                            ToolTip.text: modelData.name + " " + modelData.version
                            contentItem: RowLayout {
                                spacing: 8
                                Label {
                                    text: modelData.name
                                    Layout.fillWidth: true
                                    elide: Text.ElideMiddle
                                }
                                Label {
                                    text: modelData.version
                                    font.pixelSize: Math.round(Qt.font({}).pixelSize * 0.85)
                                    opacity: 0.6
                                }
                            }
                            onClicked: pkg.select(modelData.name)
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8
                        visible: pkg.zeroHit

                        Label {
                            text: "Aucun paquet trouvé"
                            font.bold: true
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }
                        Label {
                            text: "Vérifiez l'orthographe ou affinez votre recherche."
                            wrapMode: Text.Wrap
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }
                }
            }

            GroupBox {
                title: "Fiche paquet"
                Layout.fillWidth: true
                visible: pkg.cardJson !== ""

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Paquet"
                        fieldValue: pkg.cardJson === "" ? "" : JSON.parse(pkg.cardJson).name
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Version candidate"
                        fieldValue: pkg.cardJson === "" ? "" : String(JSON.parse(pkg.cardJson).candidate_version)
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Version installée"
                        fieldValue: pkg.cardJson === "" ? "" : String(JSON.parse(pkg.cardJson).installed_version)
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Source"
                        fieldValue: pkg.cardJson === "" ? "" : String(JSON.parse(pkg.cardJson).source)
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Taille de téléchargement"
                        fieldValue: pkg.cardJson === "" ? "" : String(JSON.parse(pkg.cardJson).download_size)
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Taille installée"
                        fieldValue: pkg.cardJson === "" ? "" : String(JSON.parse(pkg.cardJson).installed_size)
                    }
                }
            }
        }
    }
}
