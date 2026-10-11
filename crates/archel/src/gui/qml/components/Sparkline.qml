// Sparkline: rolling-window polyline (D-09, UI-SPEC Sparkline contract).
//
// Canvas polyline over up to 60 1 Hz samples (bounded Rust-side), 2px
// stroke, line color passed by the tab (theme accent for normal
// curves). QtCharts
// is not installed; Canvas keeps zero new system dependencies.
import QtQuick

Canvas {
    id: root

    property var samples: []
    property color lineColor: "transparent"

    implicitWidth: 200
    implicitHeight: 60

    onSamplesChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()
    onLineColorChanged: requestPaint()

    onPaint: {
        var ctx = getContext("2d")
        ctx.clearRect(0, 0, width, height)
        if (!samples || samples.length === 0) {
            return
        }
        ctx.strokeStyle = lineColor
        ctx.lineWidth = 2
        ctx.lineJoin = "round"
        ctx.beginPath()
        var n = samples.length
        for (var i = 0; i < n; i++) {
            var v = Math.min(100, Math.max(0, Number(samples[i])))
            var x = n === 1 ? width : (i / (n - 1)) * width
            var y = height - (v / 100) * height
            if (i === 0) {
                ctx.moveTo(x, y)
            } else {
                ctx.lineTo(x, y)
            }
        }
        ctx.stroke()
    }
}
