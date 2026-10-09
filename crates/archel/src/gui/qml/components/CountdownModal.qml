// CountdownModal placeholder (D-16 display rollback countdown). Registered in build.rs now so later GUI plans
// fill the modal body without touching the qrc list (Pitfall 2).
import QtQuick
import QtQuick.Controls

Dialog {
    modal: true
}
