// Applications tab: installed list, live search, package card and the
// write toolbar (plan 06-06). Reads stay the 06-05 shape (sequence
// tags, banner, N/A contract); writes go through PkgModel invokables
// only - the tab never talks to a subprocess, the modal or pkexec
// directly. The confirm/stream/error choreography lives in the shared
// modal session (Main.qml layer, D-12/D-13/D-15).
//
// cxx-qt registers qproperties verbatim, so every model binding below
// uses the snake_case names from bridge_pkg.rs (the invokables keep
// their camelCase cxx_name).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    id: root

    PkgModel {
        id: pkg
    }

    // Batch selection, keyed by package name so the 1 Hz installed
    // refresh never loses the check marks. selectionRev is a bump
    // counter read by isSelected() to force delegate re-evaluation.
    property var selectedNames: []
    property int selectionRev: 0

    function isSelected(name) {
        void (root.selectionRev);
        return root.selectedNames.indexOf(name) >= 0;
    }
    function setSelected(name, on) {
        var next = root.selectedNames.filter(function (n) { return n !== name; });
        if (on) {
            next.push(name);
        }
        root.selectedNames = next;
        root.selectionRev += 1;
    }
    function clearSelection() {
        root.selectedNames = [];
        root.selectionRev += 1;
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
                bannerVisible: pkg.banner_visible
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
                        onAccepted: {
                            root.clearSelection();
                            pkg.search(text);
                        }
                    }

                    BusyIndicator {
                        running: pkg.search_busy
                        visible: pkg.search_busy
                        Layout.alignment: Qt.AlignHCenter
                    }

                    ListView {
                        id: pkgList
                        Layout.fillWidth: true
                        Layout.preferredHeight: contentHeight
                        interactive: false
                        clip: true
                        model: {
                            if (pkg.search_json !== "")
                                return JSON.parse(pkg.search_json);
                            if (pkg.installed_json === "")
                                return [];
                            return JSON.parse(pkg.installed_json);
                        }
                        delegate: ItemDelegate {
                            width: pkgList.width
                            hoverEnabled: true
                            ToolTip.visible: hovered
                            ToolTip.text: modelData.name + " " + modelData.version
                            contentItem: RowLayout {
                                spacing: 8
                                CheckBox {
                                    checked: root.isSelected(modelData.name)
                                    onToggled: root.setSelected(modelData.name, checked)
                                    Accessible.name: "Sélectionner " + modelData.name
                                }
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
                        visible: pkg.zero_hit

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

            // Write toolbar (D-12/D-15): while a modal session is open
            // every button is disabled and the busy label names the
            // running transaction. The plan's "Mettre à jour ({n})"
            // count is not in the read model - honest label, no invented
            // count (documented deviation).
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Button {
                    text: "Installer"
                    highlighted: true
                    Layout.minimumWidth: 110
                    enabled: !pkg.operation_busy && root.selectedNames.length > 0
                    onClicked: {
                        pkg.installStart(JSON.stringify(root.selectedNames));
                        root.clearSelection();
                    }
                }
                Button {
                    text: "Mettre à jour"
                    Layout.minimumWidth: 130
                    enabled: !pkg.operation_busy
                    onClicked: pkg.upgradeStart()
                }
                Button {
                    text: "Supprimer"
                    Layout.minimumWidth: 110
                    enabled: !pkg.operation_busy && root.selectedNames.length > 0
                    onClicked: {
                        pkg.removeStart(JSON.stringify(root.selectedNames));
                        root.clearSelection();
                    }
                }
                Item { Layout.fillWidth: true }
                BusyIndicator {
                    running: pkg.operation_busy
                    visible: pkg.operation_busy
                    implicitHeight: implicitIndicatorSize
                }
                Label {
                    visible: pkg.operation_busy
                    text: pkg.operation_kind === "install" ? "Installation en cours…"
                          : pkg.operation_kind === "remove" ? "Suppression en cours…"
                          : pkg.operation_kind === "upgrade" ? "Mise à jour en cours…"
                          : "Opération en cours…"
                }
            }

            GroupBox {
                title: "Fiche paquet"
                Layout.fillWidth: true
                visible: pkg.card_json !== ""

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Paquet"
                        fieldValue: pkg.card_json === "" ? "" : JSON.parse(pkg.card_json).name
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Version candidate"
                        fieldValue: pkg.card_json === "" ? "" : String(JSON.parse(pkg.card_json).candidate_version)
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Version installée"
                        fieldValue: pkg.card_json === "" ? "" : String(JSON.parse(pkg.card_json).installed_version)
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Source"
                        fieldValue: pkg.card_json === "" ? "" : String(JSON.parse(pkg.card_json).source)
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Taille de téléchargement"
                        fieldValue: pkg.card_json === "" ? "" : String(JSON.parse(pkg.card_json).download_size)
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Taille installée"
                        fieldValue: pkg.card_json === "" ? "" : String(JSON.parse(pkg.card_json).installed_size)
                    }
                }
            }
        }
    }
}
