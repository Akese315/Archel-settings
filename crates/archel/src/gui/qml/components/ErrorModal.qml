// ErrorModal: the single error voice of the shell (D-17). The body text
// is always core-produced readable French (`ArchelError::message()`),
// never raw stderr - the GUI never composes error prose (thin-view
// contract). Pure view over the ShellModel modal session (plan 06-06):
// no self-closing (NoAutoClose keeps the Main.qml visible binding alive).
// `dismissed` (not `closed`): Popup already provides closed().
// Frozen api: {errorText, dismissed}.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    property string errorText: ""
    signal dismissed()

    modal: true
    width: 420
    padding: 16
    closePolicy: Popup.NoAutoClose
    // Popup/Dialog has no onEscPressed (and Keys only attaches to
    // Items, not to Popup) - a window-context Shortcut carries the
    // key instead; NoAutoClose still blocks any implicit close.
    Shortcut {
        enabled: root.visible
        sequence: "Escape"
        onActivated: root.dismissed()
    }

    // Accent-colored title via a custom header (UI-SPEC color discipline:
    // palette role, not a literal hex). `brightText` lives on the
    // Controls `palette` every Popup carries, and that palette is
    // theme-bound: Main.qml sets the window palette from ThemeModel
    // and Qt propagates it into this Dialog, so the semantic
    // warning-ink role follows the active theme like every other
    // control here.
    header: Label {
        text: "Une erreur est survenue"
        font.bold: true
        color: root.palette.brightText
        padding: 16
        bottomPadding: 0
    }

    contentHeight: col.implicitHeight

    ColumnLayout {
        id: col
        width: parent.width
        spacing: 16

        // Selectable error body: read-only TextEdit styled as a label
        // (Label has no text selection; no SelectableLabel component
        // exists in this codebase).
        TextEdit {
            Layout.fillWidth: true
            text: root.errorText
            wrapMode: TextEdit.Wrap
            readOnly: true
            selectByMouse: true
            selectByKeyboard: true
            color: root.palette.windowText
            selectionColor: root.palette.highlight
            selectedTextColor: root.palette.highlightedText
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8

            ArchelButton {
                id: fermerButton
                text: "Fermer"
                Layout.minimumWidth: 96
                highlighted: true
                // Initial keyboard focus (UI-SPEC keyboard contract):
                // Dialog has no defaultButton property, the focused
                // button carries Enter/Space activation.
                focus: true
                onClicked: root.dismissed()
            }
        }
    }
}
