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

            // Hero block: 2xl/3xl page-level spacing (UI-SPEC Landing row).
            ColumnLayout {
                Layout.fillWidth: true
                Layout.topMargin: 48
                spacing: 8

                Label {
                    text: "Accueil"
                    font.bold: true
                    font.pixelSize: Math.round(Qt.font({}).pixelSize * 1.6)
                }
                Label {
                    text: "Archel Réglages"
                    font.pixelSize: Math.round(Qt.font({}).pixelSize * 0.85)
                }
            }

            PollBanner {
                bannerVisible: landing.bannerVisible
                onDismissRequested: landing.dismissBanner()
            }

            GroupBox {
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
                        font.pixelSize: Math.round(Qt.font({}).pixelSize * 0.85)
                        wrapMode: Text.Wrap
                        Layout.fillWidth: true
                    }
                }
            }

            GroupBox {
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
