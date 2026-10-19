import AppKit

/// 输入：输出像素尺寸；返回：圆角蓝色终端与跳转箭头的 PNG 数据。
func renderIcon(size: Int) -> Data {
    let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    let scale = CGFloat(size) / 1024
    let transform = NSAffineTransform()
    transform.scale(by: scale)
    transform.concat()
    let background = NSBezierPath(roundedRect: NSRect(x: 56, y: 56, width: 912, height: 912), xRadius: 208, yRadius: 208)
    NSGradient(starting: NSColor(srgbRed: 0.08, green: 0.16, blue: 0.30, alpha: 1),
               ending: NSColor(srgbRed: 0.15, green: 0.43, blue: 0.79, alpha: 1))!.draw(in: background, angle: 45)
    let terminal = NSBezierPath(roundedRect: NSRect(x: 204, y: 245, width: 616, height: 534), xRadius: 56, yRadius: 56)
    NSColor.white.withAlphaComponent(0.92).setStroke()
    terminal.lineWidth = 30
    terminal.stroke()
    let prompt = NSBezierPath()
    prompt.move(to: NSPoint(x: 290, y: 603))
    prompt.line(to: NSPoint(x: 385, y: 515))
    prompt.line(to: NSPoint(x: 290, y: 427))
    prompt.move(to: NSPoint(x: 444, y: 426))
    prompt.line(to: NSPoint(x: 537, y: 426))
    prompt.lineWidth = 38
    prompt.lineCapStyle = .round
    prompt.lineJoinStyle = .round
    prompt.stroke()
    let arrow = NSBezierPath()
    arrow.move(to: NSPoint(x: 570, y: 310))
    arrow.line(to: NSPoint(x: 865, y: 605))
    arrow.move(to: NSPoint(x: 665, y: 605))
    arrow.line(to: NSPoint(x: 865, y: 605))
    arrow.line(to: NSPoint(x: 865, y: 405))
    NSColor(srgbRed: 0.37, green: 0.95, blue: 0.78, alpha: 1).setStroke()
    arrow.lineWidth = 62
    arrow.lineCapStyle = .round
    arrow.lineJoinStyle = .round
    arrow.stroke()
    NSGraphicsContext.restoreGraphicsState()
    return bitmap.representation(using: .png, properties: [:])!
}

let output = URL(fileURLWithPath: CommandLine.arguments[1])
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
for size in [16, 32, 128, 256, 512] {
    for density in [1, 2] {
        let suffix = density == 1 ? "" : "@2x"
        try renderIcon(size: size * density).write(to: output.appendingPathComponent("icon_\(size)x\(size)\(suffix).png"))
    }
}
