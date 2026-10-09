// ErrorModal: the single error voice of the shell (D-17). The body text
// is always core-produced readable French (`ArchelError::message()`),
// never raw stderr - the GUI never composes error prose (thin-view
// contract). Pure view over the ShellModel modal session (plan 06-06):
// no self-closing (NoAutoClose keeps the Main.qml visible binding alive).
// Frozen api: {errorText, closed}.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    property string errorText: ""
    signal closed()

    modal: true
    width: 420
    padding: 16
    closePolicy: Popup.NoAutoClose
    onEscPressed: root.closed()

    SystemPalette {
        id: pal
    }

    // Red-accent title via a custom header (UI-SPEC color discipline:
    // palette role, not a literal hex - brightText is the semantic
    // warning-ink role of the system palette).
    header: Label {
        text: "Une erreur est survenue"
        font.bold: true
        color: pal.brightText
        padding: 16
        bottomPadding: 0
    }

    contentHeight: col.implicitHeight

    ColumnLayout {
        id: col
        width: parent.width
        spacing: 16

        SelectableLabel {
            Layout.fillWidth: true
            text: root.errorText
            wrapMode: Label.WrapAnywhere
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8

            Button {
                id: fermerButton
                text: "Fermer"
                Layout.minimumWidth: 96
                highlighted: true
                onClicked: root.closed()
            }
        }
    }

    defaultButton: fermerButton
}
