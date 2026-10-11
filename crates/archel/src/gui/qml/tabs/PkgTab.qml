// Applications tab: installed list, live search, package card and the
// write toolbar (plan 06-06). Reads stay the 06-05 shape (sequence
// tags, banner, N/A contract); writes go through PkgModel invokables
// only - the tab never talks to a subprocess, the modal or pkexec
// directly. The confirm/stream/error choreography lives in the shared
// modal session (Main.qml layer, D-12/D-13/D-15).
//
// Convention (D-13): QML binds camelCase while the bridge qproperties
// stay snake_case in bridge_pkg.rs - Qt case-folding resolves them
// (the invokables keep their camelCase cxx_name).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    id: root

    // Exposed so Main.qml (AUR trust review) can reach the model:
    // component ids are scoped to this file (GAP-1).
    property alias pkg: pkg

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
                        onAccepted: {
                            root.clearSelection();
                            pkg.search(text);
                        }
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
                    enabled: !pkg.operationBusy && root.selectedNames.length > 0
                    onClicked: {
                        pkg.installStart(JSON.stringify(root.selectedNames));
                        root.clearSelection();
                    }
                }
                Button {
                    text: "Mettre à jour"
                    Layout.minimumWidth: 130
                    enabled: !pkg.operationBusy
                    onClicked: pkg.upgradeStart()
                }
                Button {
                    text: "Supprimer"
                    Layout.minimumWidth: 110
                    enabled: !pkg.operationBusy && root.selectedNames.length > 0
                    onClicked: {
                        pkg.removeStart(JSON.stringify(root.selectedNames));
                        root.clearSelection();
                    }
                }
                // AUR is a reviewed single-package path (D-19): one
                // checked row, core review, trust block, then the
                // cancellable yay build. Repository rows stay on the
                // Installer button (the review redirects them with
                // core's own hint).
                Button {
                    text: "Installer depuis AUR"
                    Layout.minimumWidth: 170
                    enabled: !pkg.operationBusy && root.selectedNames.length === 1
                    onClicked: {
                        pkg.aurReviewStart(root.selectedNames[0]);
                        root.clearSelection();
                    }
                }
                Item { Layout.fillWidth: true }
                BusyIndicator {
                    running: pkg.operationBusy
                    visible: pkg.operationBusy
                    implicitHeight: implicitIndicatorSize
                }
                Label {
                    visible: pkg.operationBusy
                    text: pkg.operationKind === "install" ? "Installation en cours…"
                          : pkg.operationKind === "remove" ? "Suppression en cours…"
                          : pkg.operationKind === "upgrade" ? "Mise à jour en cours…"
                          : pkg.operationKind === "aur" ? "Revue AUR en cours…"
                          : pkg.operationKind === "aur-build" ? "Compilation AUR en cours…"
                          : "Opération en cours…"
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
