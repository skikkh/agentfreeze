#!/usr/bin/env swift

import AppKit
import Foundation
import ImageIO
import UniformTypeIdentifiers

struct DemoFrame {
    let file: String
    let delay: Double
}

guard CommandLine.arguments.count == 2 else {
    FileHandle.standardError.write(Data("usage: render-demo.swift <project-root>\n".utf8))
    exit(2)
}

let projectRoot = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
let output = projectRoot.appendingPathComponent("docs/assets/demo.gif")
let frames = [
    DemoFrame(file: "scan.txt", delay: 2.6),
    DemoFrame(file: "freeze.txt", delay: 2.8),
    DemoFrame(file: "drift.txt", delay: 3.6),
]
let canvas = NSSize(width: 1200, height: 675)

func color(_ hex: UInt32) -> NSColor {
    NSColor(
        calibratedRed: CGFloat((hex >> 16) & 0xff) / 255,
        green: CGFloat((hex >> 8) & 0xff) / 255,
        blue: CGFloat(hex & 0xff) / 255,
        alpha: 1
    )
}

func fill(_ rect: NSRect, with fillColor: NSColor, radius: CGFloat = 0) {
    fillColor.setFill()
    if radius == 0 {
        NSBezierPath(rect: rect).fill()
    } else {
        NSBezierPath(roundedRect: rect, xRadius: radius, yRadius: radius).fill()
    }
}

func draw(_ text: String, at point: NSPoint, color textColor: NSColor, font: NSFont) {
    NSString(string: text).draw(
        at: point,
        withAttributes: [
            .font: font,
            .foregroundColor: textColor,
        ]
    )
}

func textColor(for line: String) -> NSColor {
    if line.hasPrefix("$") {
        return color(0x79c0ff)
    }
    if line.hasPrefix("Drift") || line.hasPrefix("  ~") || line.hasPrefix("exit") {
        return color(0xff7b72)
    }
    if line.hasPrefix("Verified") {
        return color(0x56d364)
    }
    if line.hasPrefix("#") {
        return color(0x8b949e)
    }
    return color(0xc9d1d9)
}

func renderFrame(_ text: String) throws -> CGImage {
    guard let bitmap = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: Int(canvas.width),
        pixelsHigh: Int(canvas.height),
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    ), let graphics = NSGraphicsContext(bitmapImageRep: bitmap) else {
        throw NSError(domain: "AgentFreezeDemo", code: 1, userInfo: [
            NSLocalizedDescriptionKey: "cannot create bitmap context"
        ])
    }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = graphics
    defer {
        graphics.flushGraphics()
        NSGraphicsContext.restoreGraphicsState()
    }

    fill(NSRect(origin: .zero, size: canvas), with: color(0x07101f))
    fill(NSRect(x: 36, y: 36, width: 1128, height: 603), with: color(0x0d1117), radius: 18)
    fill(NSRect(x: 36, y: 581, width: 1128, height: 58), with: color(0x161b22), radius: 18)
    fill(NSRect(x: 36, y: 581, width: 1128, height: 29), with: color(0x161b22))

    for (x, dotColor) in [(68.0, 0xff5f56), (98.0, 0xffbd2e), (128.0, 0x27c93f)] {
        fill(NSRect(x: x, y: 603, width: 16, height: 16), with: color(UInt32(dotColor)), radius: 8)
    }

    let headerFont = NSFont.monospacedSystemFont(ofSize: 20, weight: .medium)
    let bodyFont = NSFont.monospacedSystemFont(ofSize: 28, weight: .regular)
    draw(
        "AgentFreeze  ·  user extension integrity",
        at: NSPoint(x: 385, y: 599),
        color: color(0x8b949e),
        font: headerFont
    )

    var y: CGFloat = 526
    for line in text.components(separatedBy: .newlines) {
        draw(line, at: NSPoint(x: 72, y: y), color: textColor(for: line), font: bodyFont)
        y -= 46
    }

    guard let result = bitmap.cgImage else {
        throw NSError(domain: "AgentFreezeDemo", code: 4, userInfo: [
            NSLocalizedDescriptionKey: "cannot create a CGImage"
        ])
    }
    return result
}

try FileManager.default.createDirectory(
    at: output.deletingLastPathComponent(),
    withIntermediateDirectories: true
)

guard let destination = CGImageDestinationCreateWithURL(
    output as CFURL,
    UTType.gif.identifier as CFString,
    frames.count,
    nil
) else {
    throw NSError(domain: "AgentFreezeDemo", code: 2, userInfo: [
        NSLocalizedDescriptionKey: "cannot create GIF destination"
    ])
}

CGImageDestinationSetProperties(destination, [
    kCGImagePropertyGIFDictionary: [
        kCGImagePropertyGIFLoopCount: 0
    ]
] as CFDictionary)

for frame in frames {
    let source = projectRoot.appendingPathComponent("scripts/demo/\(frame.file)")
    let text = try String(contentsOf: source, encoding: .utf8)
    let image = try renderFrame(text)
    CGImageDestinationAddImage(destination, image, [
        kCGImagePropertyGIFDictionary: [
            kCGImagePropertyGIFDelayTime: frame.delay
        ]
    ] as CFDictionary)
}

guard CGImageDestinationFinalize(destination) else {
    throw NSError(domain: "AgentFreezeDemo", code: 3, userInfo: [
        NSLocalizedDescriptionKey: "cannot finalize GIF"
    ])
}

print("Rendered \(output.path)")
