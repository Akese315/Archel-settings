// Accueil tab: GNOME-Settings-like overview — update counts from the
// cached path plus the version/kernel snapshot (D-07, D-22).
//
// The Timer runs unconditionally from launch (this is the default tab)
// and is never gated on visibility — returning to the tab always shows
// fresh data (D-03/D-06). Pre-first-tick values read the ellipsis token;
// failures read N/A plus the once-per-episode banner (D-04/D-05).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.archel.settings

Item {
    LandingModel {
        id: landing
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: landing.tick()
    }
    Component.onCompleted: landing.tick()

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
                pageTitle: "Accueil"
                pageSubtitle: "État général du système"
            }

            PollBanner {
                bannerVisible: landing.bannerVisible
                onDismissRequested: landing.dismissBanner()
            }

            SectionCard {
                title: "Mises à jour"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "En attente"
                        fieldValue: landing.updatesText
                    }
                    Label {
                        visible: landing.notesText !== ""
                        text: landing.notesText
                        font.pixelSize: Math.round(Qt.application.font.pixelSize * 0.85)
                        wrapMode: Text.Wrap
                        Layout.fillWidth: true
                    }
                }
            }

            SectionCard {
                title: "Système"
                Layout.fillWidth: true

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Version"
                        fieldValue: landing.versionText
                    }
                    StatField {
                        Layout.fillWidth: true
                        fieldLabel: "Noyau"
                        fieldValue: landing.kernelText
                    }
                }
            }
        }
    }
}
