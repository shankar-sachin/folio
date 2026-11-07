import Cocoa
// Rebuild the macOS icon family from the included, letter-free artwork.
let input = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "native/Folio-Icon.png"
let output = CommandLine.arguments.count > 2 ? CommandLine.arguments[2] : ".build/Folio.iconset"
guard let artwork = NSImage(contentsOfFile: input) else { fatalError("Cannot load icon artwork") }
try FileManager.default.createDirectory(atPath:output,withIntermediateDirectories:true)
let labels: [Int:[String]] = [16:["icon_16x16.png"],32:["icon_16x16@2x.png","icon_32x32.png"],64:["icon_32x32@2x.png"],128:["icon_128x128.png"],256:["icon_128x128@2x.png","icon_256x256.png"],512:["icon_256x256@2x.png","icon_512x512.png"],1024:["icon_512x512@2x.png"]]
for size in labels.keys.sorted() {
    let bitmap = NSBitmapImageRep(bitmapDataPlanes:nil,pixelsWide:size,pixelsHigh:size,bitsPerSample:8,samplesPerPixel:4,hasAlpha:true,isPlanar:false,colorSpaceName:.deviceRGB,bytesPerRow:0,bitsPerPixel:0)!
    bitmap.size = NSSize(width:size,height:size)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep:bitmap)
    NSGraphicsContext.current?.imageInterpolation = .high
    artwork.draw(in:NSRect(x:0,y:0,width:size,height:size),from:.zero,operation:.copy,fraction:1)
    NSGraphicsContext.restoreGraphicsState()
    let data = bitmap.representation(using:.png,properties:[:])!
    for label in labels[size]! { try data.write(to:URL(fileURLWithPath:output).appendingPathComponent(label)) }
}
