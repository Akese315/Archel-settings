// StreamModal (D-13/D-14): the one privileged-operation view. Terminal-
// like by monospace + palette.base only - never a hardcoded dark theme
// (D-24 system-follow). Pure view over the ShellModel modal session
// (plan 06-06): lines arrive via Mutex + Timer-poll into `lines`; the
// footer is Annuler while running and Fermer once done. D-15: while
// !done the only enabled control is Annuler and Esc is blocked - an
// interruption is an explicit decision, never a disappearance. The end
// state is data, not an error (D-14): operationResult carries the ASCII
// key ("completed"/"cancelled"/"failed") and the French sentence lives
// here. `operationResult` (not `result`) and `dismissed` (not `closed`):
// Dialog/Popup already provide final result + closed().
// The trust block (D-19 summary card + PKGBUILD expander + trust
// checkbox + Valider) is added by Task 3 of the same plan on these same
// frozen properties. Frozen api: {lines, done, operationResult,
// trustNeeded, trustSummary, pkgbuild, pkgbuildSeen, trustConfirmed,
// cancelRequested, dismissed, aurValidated, pkgbuildSeenRequested,
// trustToggled}.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    property string lines: ""
    property bool done: false
    property string operationResult: ""
    // D-19 trust-gate surface (rendered by Task 3; carried now so the
    // api block stays frozen for 06-07/08).
    property bool trustNeeded: false
    property string trustSummary: ""
    property string pkgbuild: ""
    property bool pkgbuildSeen: false
    property bool trustConfirmed: false

    signal cancelRequested()
    signal dismissed()
    signal aurValidated()
    signal pkgbuildSeenRequested()
    signal trustToggled(bool checked)

    modal: true
    title: "Opération en cours"
    width: 560
    padding: 16
    closePolicy: Popup.NoAutoClose
    // D-15: Esc cannot dismiss a running operation; once done it acts
    // as Fermer.
    onEscPressed: {
        if (root.done) {
            root.dismissed();
        }
    }

    SystemPalette {
        id: pal
    }

    contentHeight: col.implicitHeight

    ColumnLayout {
        id: col
        width: parent.width
        spacing: 16

        ScrollView {
            id: logView
            Layout.fillWidth: true
            Layout.preferredHeight: 240
            clip: true
            background: Rectangle {
                color: pal.base
            }

            TextEdit {
                id: logText
                readOnly: true
                text: root.lines
                wrapMode: TextEdit.WrapAnywhere
                font.family: "monospace"
                font.pointSize: 13
                color: pal.text
                selectionColor: pal.highlight
                // Auto-scroll to the newest line (UI-SPEC streaming-log
                // overflow row); the buffer is capped core-side, this is
                // just the viewport follow.
                width: logView.availableWidth > 0 ? logView.availableWidth : root.width - 2 * root.padding
                onTextChanged: logView.contentItem.contentY = Math.max(
                    0, logView.contentItem.contentHeight - logView.height)
            }
        }

        ProgressBar {
            Layout.fillWidth: true
            visible: !root.done
            indeterminate: true
        }

        // D-14: the end of the transaction is reported, never collapsed
        // into an error - a cancelled stop reads as cancelled.
        Label {
            Layout.fillWidth: true
            visible: root.done
            text: root.operationResult === "completed" ? "Opération terminée."
                  : root.operationResult === "cancelled" ? "Opération annulée (interrompue par l’utilisateur)."
                  : "Opération échouée."
        }

        // D-19 AUR trust gate (summary-first form of Phase 3's full
        // display, D-05): the summary card renders the core-prepared
        // trustSummary JSON (core labels, honest N/A, no GUI-invented
        // copy), the raw PKGBUILD sits behind a ONE-WAY expander - the
        // first opening latches pkgbuildSeen through the session and
        // collapsing never un-latches it - and Valider is enabled only
        // once both the viewing and the explicit trust are given. The
        // Rust worker re-checks the same gate; this binding is UX,
        // never the security boundary.
        ColumnLayout {
            id: trustBlock
            Layout.fillWidth: true
            spacing: 10
            visible: root.trustNeeded

            Repeater {
                model: root.trustSummary === "" ? [] : [JSON.parse(root.trustSummary)]
                delegate: ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: cardCol.implicitHeight + 20
                        radius: 6
                        color: pal.base
                        border.color: pal.mid
                        ColumnLayout {
                            id: cardCol
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.margins: 10
                            spacing: 4
                            Label {
                                text: modelData.name + "  " + modelData.version
                                font.bold: true
                            }
                            Label {
                                text: "Source : " + modelData.source
                                opacity: 0.8
                            }
                            Label {
                                text: (modelData.maintainerLabel + " : "
                                       + (modelData.maintainer ? modelData.maintainer : "N/A"))
                            }
                            Label {
                                text: (modelData.votesLabel + " : "
                                       + (modelData.votes ? modelData.votes : "N/A"))
                            }
                            Label {
                                text: (modelData.popularityLabel + " : "
                                       + (modelData.popularity ? modelData.popularity : "N/A"))
                            }
                            Label {
                                text: (modelData.outdatedLabel + " : "
                                       + (modelData.outOfDate ? modelData.outOfDate : "N/A"))
                            }
                        }
                    }

                    Label {
                        visible: (modelData.dependencies || []).length > 0
                        text: modelData.dependenciesLabel
                        font.bold: true
                    }
                    Label {
                        visible: (modelData.dependencies || []).length > 0
                        Layout.fillWidth: true
                        text: (modelData.dependencies || []).join(", ")
                        wrapMode: Label.WordWrap
                    }
                    Label {
                        visible: (modelData.warnings || []).length > 0
                        text: modelData.warningsLabel
                        font.bold: true
                        color: "#c0392b"
                    }
                    Label {
                        visible: (modelData.warnings || []).length > 0
                        Layout.fillWidth: true
                        text: (modelData.warnings || []).join("\n")
                        wrapMode: Label.WordWrap
                        color: "#c0392b"
                    }
                }
            }

            Button {
                id: pkgbuildExpander
                checkable: true
                text: checked ? "Masquer le PKGBUILD" : "Voir le PKGBUILD"
                // One-way: only the opening reports a viewing; the
                // session ignores any collapse (no reset path exists).
                onCheckedChanged: if (checked)
                    root.pkgbuildSeenRequested()
            }

            ScrollView {
                Layout.fillWidth: true
                Layout.preferredHeight: 160
                clip: true
                visible: pkgbuildExpander.checked
                background: Rectangle {
                    color: pal.base
                }
                TextEdit {
                    readOnly: true
                    text: root.pkgbuild
                    wrapMode: TextEdit.WrapAnywhere
                    font.family: "monospace"
                    font.pointSize: 12
                    color: pal.text
                    selectionColor: pal.highlight
                    width: trustBlock.width > 0 ? trustBlock.width : 400
                }
            }

            CheckBox {
                id: trustCheck
                text: "Je confirme la source"
                checked: root.trustConfirmed
                onToggled: root.trustToggled(checked)
            }

            RowLayout {
                Layout.alignment: Qt.AlignRight
                Button {
                    id: validateButton
                    Layout.minimumWidth: 96
                    highlighted: true
                    text: "Valider"
                    // D-19: neither flag alone suffices - the expander
                    // must have been opened at least once AND explicit
                    // trust given. Rust-side worker re-checks these
                    // same two session flags before any yay call.
                    enabled: root.pkgbuildSeen && root.trustConfirmed
                    onClicked: root.aurValidated()
                }
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8

            Button {
                id: footerButton
                Layout.minimumWidth: 96
                highlighted: true
                // Initial keyboard focus (UI-SPEC keyboard contract):
                // Dialog has no defaultButton property, the focused
                // button carries Enter/Space activation.
                focus: true
                text: root.done ? "Fermer" : "Annuler"
                // D-15 dismissal path: Annuler stays available during
                // the AUR review too (Esc is blocked, NoAutoClose) -
                // session.dismiss_or_cancel routes it to a dismiss
                // there, Valider is the forward path.
                // One footer control that swaps roles (D-15: Annuler is
                // the only enabled button while running). Annuler only
                // requests the stop; the modal stays until the worker
                // reports the end state.
                onClicked: root.done ? root.dismissed() : root.cancelRequested()
            }
        }
    }
}
