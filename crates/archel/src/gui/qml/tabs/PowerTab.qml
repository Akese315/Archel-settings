// Alimentation tab: active profile plus backend tag, with the
// plan-07 profile switch behind the D-12 confirm modal.
//
// The Timer runs unconditionally so the model keeps ticking while this
// tab is hidden (D-03/D-06). Pre-first-tick values read the ellipsis
// token; failures read N/A plus the once-per-episode banner (D-04/D-05).
// The segmented control calls power.setProfile on click: the question
// is the core's own confirm_set_prompt verbatim, and while the switch
// is in flight the control shows a spinner and stays disabled (D-15) -
// every other tab stays usable. The refresh after the switch reads the
// new profile back from PPD.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    PowerModel {
        id: power
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: power.tick()
    }
    Component.onCompleted: power.tick()

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
                pageTitle: "Alimentation"
                pageSubtitle: "Profils d'énergie et pilote"
            }

            PollBanner {
                bannerVisible: power.bannerVisible
                onDismissRequested: power.dismissBanner()
            }

            SectionCard {
                title: "Profil"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    RowLayout {
                        spacing: 8

                        Repeater {
                            model: power.profilesJson === "" ? [] : JSON.parse(power.profilesJson)
                            delegate: RadioButton {
                                text: modelData.id
                                checked: modelData.active
                                enabled: !power.operationBusy
                                onClicked: power.setProfile(modelData.id)
                            }
                        }
                        BusyIndicator {
                            visible: power.operationBusy
                            running: power.operationBusy
                        }
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Profil actif"
                        fieldValue: power.profileText
                    }
                }
            }

            SectionCard {
                title: "Pilote"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Backend"
                        fieldValue: power.backendText
                    }
                    StatField {
                        Layout.fillWidth: true
                        visible: power.noteText !== ""
                        fieldLabel: "Note"
                        fieldValue: power.noteText
                    }
                }
            }
        }
    }
}
