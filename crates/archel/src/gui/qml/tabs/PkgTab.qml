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

    // ── Installed-list browsing (filters by name and size) ────────
    // Remote search results keep precedence exactly as before; the
    // name/size filters and sorts only apply to the installed list.

    // Live parsed rows: search hits first (name/version/source), the
    // installed list otherwise (name/version/size).
    readonly property var allRows: {
        if (pkg.searchJson !== "")
            return JSON.parse(pkg.searchJson);
        if (pkg.installedJson === "")
            return [];
        return JSON.parse(pkg.installedJson);
    }

    property int sortMode: 0   // 0 nom ↑, 1 nom ↓, 2 taille ↑, 3 taille ↓
    property int sizeFilter: 0 // 0 toutes, 1 ≥100 Mo, 2 ≥1 Go, 3 ≤10 Mo

    function filteredRows() {
        var rows = root.allRows;
        var searching = pkg.searchJson !== "";
        var query = filterField.text.trim().toLowerCase();
        var out = [];
        for (var i = 0; i < rows.length; i++) {
            var row = rows[i];
            if (query !== "" && row.name.toLowerCase().indexOf(query) < 0)
                continue;
            if (!searching) {
                var size = row.size || 0;
                if (root.sizeFilter === 1 && size < 100 * 1024 * 1024)
                    continue;
                if (root.sizeFilter === 2 && size < 1024 * 1024 * 1024)
                    continue;
                if (root.sizeFilter === 3 && size > 10 * 1024 * 1024)
                    continue;
            }
            out.push(row);
        }
        if (!searching) {
            if (root.sortMode === 0)
                out.sort(function (a, b) { return a.name.localeCompare(b.name); });
            else if (root.sortMode === 1)
                out.sort(function (a, b) { return b.name.localeCompare(a.name); });
            else if (root.sortMode === 2)
                out.sort(function (a, b) { return (a.size || 0) - (b.size || 0); });
            else
                out.sort(function (a, b) { return (b.size || 0) - (a.size || 0); });
        }
        return out;
    }

    // Human-readable size for the installed rows (search rows carry
    // no size: rendered blank, never a fake 0).
    function formatBytes(n) {
        if (n === null || n === undefined)
            return "";
        if (n < 1024)
            return n + " o";
        if (n < 1024 * 1024)
            return (n / 1024).toFixed(1) + " Kio";
        if (n < 1024 * 1024 * 1024)
            return (n / 1024 / 1024).toFixed(1) + " Mio";
        return (n / 1024 / 1024 / 1024).toFixed(2) + " Gio";
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

            PageHeader {
                Layout.fillWidth: true
                pageTitle: "Applications"
                pageSubtitle: "Paquets et mises à jour"
            }

            PollBanner {
                bannerVisible: pkg.bannerVisible
                onDismissRequested: pkg.dismissBanner()
            }

            SectionCard {
                title: "Paquets"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    TextField {
                        id: searchField
                        Layout.fillWidth: true
                        placeholderText: "Rechercher un paquet à installer…"
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

                    // Name filter + size filter + sort over the
                    // installed list (client-side; the 1 Hz refresh
                    // keeps the same filter alive).
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        TextField {
                            id: filterField
                            Layout.fillWidth: true
                            placeholderText: "Filtrer la liste installée…"
                            selectByMouse: true
                        }
                        Label { text: "Taille" }
                        ComboBox {
                            id: sizeFilterBox
                            Layout.preferredWidth: 130
                            model: ["Toutes", "≥ 100 Mo", "≥ 1 Go", "≤ 10 Mo"]
                            onActivated: function (index) { root.sizeFilter = index }
                        }
                        Label { text: "Tri" }
                        ComboBox {
                            id: sortBox
                            Layout.preferredWidth: 170
                            model: ["Nom (A→Z)", "Nom (Z→A)", "Taille (croissante)", "Taille (décroissante)"]
                            onActivated: function (index) { root.sortMode = index }
                        }
                    }

                    // Write toolbar (D-12/D-15): sits ABOVE the list so
                    // it stays reachable with a thousand rows loaded.
                    // While a modal session is open every button is
                    // disabled and the busy label names the running
                    // transaction.
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
                            // Explicit size: BusyIndicator exposes no
                            // implicitIndicatorSize property (that name
                            // was a ReferenceError); 24 px keeps the
                            // toolbar row at button height.
                            implicitWidth: 24
                            implicitHeight: 24
                            running: pkg.operationBusy
                            visible: pkg.operationBusy
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

                    ListView {
                        id: pkgList
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.min(contentHeight, 320)
                        interactive: contentHeight > 320
                        clip: true
                        model: root.filteredRows()
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
                                    text: root.formatBytes(modelData.size)
                                    Layout.preferredWidth: 80
                                    horizontalAlignment: Text.AlignRight
                                    opacity: 0.6
                                    font.pixelSize: Math.round(Qt.application.font.pixelSize * 0.85)
                                }
                                Label {
                                    text: modelData.version
                                    font.pixelSize: Math.round(Qt.application.font.pixelSize * 0.85)
                                    opacity: 0.6
                                }
                            }
                            onClicked: pkg.select(modelData.name)
                        }
                    }

                    Label {
                        Layout.fillWidth: true
                        visible: pkg.searchJson === ""
                        opacity: 0.6
                        text: root.filteredRows().length + " paquet(s) affiché(s)"
                        font.pixelSize: Math.round(Qt.application.font.pixelSize * 0.85)
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

            SectionCard {
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
