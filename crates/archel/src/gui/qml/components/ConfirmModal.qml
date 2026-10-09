// ConfirmModal placeholder (D-12 confirm gate). Registered in build.rs now so later GUI plans
// fill the modal body without touching the qrc list (Pitfall 2).
import QtQuick
import QtQuick.Controls

Dialog {
    modal: true
}
