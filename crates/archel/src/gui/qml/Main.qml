// Archel settings shell (D-22): 200px vertical rail of 10 icon+label
// TabButtons over a content StackLayout, replacing the Phase-1 spike.
//
// Pattern (proven in Spike.qml, retired by this file): QtQuick.Controls
// TabBar is horizontal-only, so vertical tabs are a Column of TabButtons
// driving a StackLayout currentIndex. The Column lives in its own
// Flickable so ten buttons taller than the window scroll instead of
// clipping (UI-SPEC rail overflow row). Tab labels are French in the
// locked order (D-25); geometry persistence arrives with the ShellModel.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: window
    visible: true
    width: 900
    height: 620
    minimumWidth: 720
    minimumHeight: 480
    title: "Archel Réglages"

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Rail: fixed 200px, scrolling its own Flickable (UI-SPEC rail).
        Flickable {
            Layout.preferredWidth: 200
            Layout.fillHeight: true
            contentHeight: rail.height
            boundsBehavior: Flickable.StopAtBounds
            clip: true

            Column {
                id: rail
                width: 200
                spacing: 4

                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    checked: true
                    onClicked: stack.currentIndex = 0
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: ""
                        }
                        Label {
                            text: "Accueil"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 1
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: ""
                        }
                        Label {
                            text: "Réseau"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 2
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: ""
                        }
                        Label {
                            text: "Alimentation"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 3
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: ""
                        }
                        Label {
                            text: "Bluetooth"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 4
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: ""
                        }
                        Label {
                            text: "Apparence"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 5
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: ""
                        }
                        Label {
                            text: "Son"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 6
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: ""
                        }
                        Label {
                            text: "Applications"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 7
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: ""
                        }
                        Label {
                            text: "Affichage"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 8
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: ""
                        }
                        Label {
                            text: "Périphériques"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
                TabButton {
                    width: rail.width
                    height: 32
                    checkable: true
                    autoExclusive: true
                    onClicked: stack.currentIndex = 9
                    contentItem: RowLayout {
                        spacing: 4
                        Image {
                            Layout.preferredWidth: 16
                            Layout.preferredHeight: 16
                            sourceSize.width: 16
                            sourceSize.height: 16
                            source: ""
                        }
                        Label {
                            text: "Système"
                            Layout.fillWidth: true
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
            }
        }

        // Tab content: one registered file per tab; hidden tabs are not
        // rendered (D-03). Landing (Accueil) is the default page (D-22).
        StackLayout {
            id: stack
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: 0

            LandingTab {}
            NetTab {}
            PowerTab {}
            BtTab {}
            AppearanceTab {}
            AudioTab {}
            PkgTab {}
            DisplayTab {}
            InputTab {}
            SystemTab {}
        }
    }
}
