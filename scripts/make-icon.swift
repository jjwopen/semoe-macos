// 메뉴 막대용 템플릿 아이콘(icon.tiff)을 만든다. 사용: swift scripts/make-icon.swift <출력 경로>
import AppKit

func rep(_ px: Int) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: px, pixelsHigh: px, bitsPerSample: 8,
                               samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = NSSize(width: 16, height: 16)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let text = NSAttributedString(string: "세", attributes: [
        .font: NSFont.systemFont(ofSize: 13, weight: .semibold),
        .foregroundColor: NSColor.black,
    ])
    let size = text.size()
    text.draw(at: NSPoint(x: (16 - size.width) / 2, y: (16 - size.height) / 2))
    NSGraphicsContext.restoreGraphicsState()
    return rep
}

let image = NSImage(size: NSSize(width: 16, height: 16))
image.addRepresentation(rep(16))
image.addRepresentation(rep(32))
try! image.tiffRepresentation!.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
