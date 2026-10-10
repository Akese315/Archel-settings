// VpnImportPopup (D-20): single popup importing a VPN configuration
// file - .ovpn (OpenVPN) or .conf (WireGuard) - or a typed path.
//
// Pure view: it collects a path and emits importRequested(path); the
// NetModel worker validates, confirms and imports through the core
// (the popup never touches the filesystem or nmcli itself). Both
// picker paths exist side by side: the FileDialog button plus a typed
// path field with its own button, so a missing platform file dialog
// (A2) degrades to typing instead of dead-ending. While an import
// runs the buttons disable themselves (D-15); the result banner lives
// in the NetTab, failures in the global ErrorModal.
// Frozen api: {busy, importRequested(path), dismissed}.
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

Popup {
    id: root

    property bool busy: false
    signal importRequested(string path)
    signal dismissed()

    modal: true
    width: 480
    padding: 16

    FileDialog {
        id: filePicker
        title: "Importer une configuration VPN"
        nameFilters: ["Configs VPN (*.ovpn *.conf)", "OpenVPN (*.ovpn)", "WireGuard (*.conf)"]
        onAccepted: pathField.text = selectedFile.toString().replace("file://", "")
    }

    ColumnLayout {
        width: parent.width
        spacing: 16

        Label {
            text: "Importer un VPN"
            font.bold: true
            Layout.fillWidth: true
        }

        Label {
            text: "Choisissez un fichier de configuration .ovpn (OpenVPN) ou .conf (WireGuard). Le fichier est vérifié avant l'import."
            wrapMode: Label.WrapAnywhere
            Layout.fillWidth: true
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Button {
                text: "Parcourir…"
                enabled: !root.busy
                onClicked: filePicker.open()
            }
            BusyIndicator {
                visible: root.busy
                running: root.busy
            }
        }

        Label {
            text: "Ou saisissez le chemin directement :"
            Layout.fillWidth: true
        }

        TextField {
            id: pathField
            Layout.fillWidth: true
            enabled: !root.busy
            placeholderText: "/home/utilisateur/bureau.ovpn"
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8

            Button {
                text: "Fermer"
                Layout.minimumWidth: 96
                enabled: !root.busy
                onClicked: root.dismissed()
            }
            Button {
                text: "Importer"
                Layout.minimumWidth: 96
                highlighted: true
                enabled: !root.busy && pathField.text.trim() !== ""
                onClicked: root.importRequested(pathField.text.trim())
            }
        }
    }
}
