import Foundation
import CoreGraphics
import AppKit

@main
enum InputTests {
    static func main() throws {
        for text in ["默认", "禁用", "Cmd+C", "Command+Shift+Z", "Option+Left", "ctrl + 2", "F20"] {
            _ = try ShortcutParser.parse(text)
        }
        for text in ["水平滚动", "", "Cmd", "Cmd+", "Cmd+Command+C", "Alt+Option+C", "Ctrl+C+V", "F21", "Ctrl++", "手势按钮", "打开应用:NotAnAbsolute.app"] {
            do {
                _ = try ShortcutParser.parse(text)
                fatalError("Parser unexpectedly accepted \(text)")
            } catch {}
        }
        guard case .shortcut(let key, let flags) = try ShortcutParser.parse("Command+Option+Shift+C"),
              key == 8, flags == [.maskCommand,.maskAlternate,.maskShift] else {
            fatalError("macOS shortcut translation is wrong")
        }
        guard case .shortcut(let deleteKey, _) = try ShortcutParser.parse("Delete"), deleteKey == 117 else {
            fatalError("Delete must map to forward delete, not backspace")
        }
        // Older files load with both independent toggles off; retired action
        // values migrate to side settings and can never enable the middle click.
        let legacy = try JSONDecoder().decode(MouseConfig.self, from:Data(
            #"{"version":1,"enabled":true,"back":"水平滚动","forward":"Cmd+C","middle":"水平滚动"}"#.utf8))
        assert(legacy.back == "默认" && legacy.forward == "Cmd+C" && legacy.middle == "默认")
        assert(legacy.horizontalScroll.back && !legacy.horizontalScroll.forward)
        assert(!legacy.horizontalScroll.enabled(for:2))
        var hold = HeldSideButtons()
        hold.begin(button:2)
        assert(!hold.scroll(hasVerticalMotion:true))
        assert(hold.release(button:2) == nil)
        hold.begin(button:3)
        assert(!hold.scroll(hasVerticalMotion:false))
        assert(hold.release(button:3) == true) // no wheel: keep configured click
        hold.begin(button:3); hold.begin(button:4)
        assert(hold.scroll(hasVerticalMotion:true))
        assert(hold.release(button:3) == false)
        assert(hold.scroll(hasVerticalMotion:true)) // other held side is still active
        assert(hold.release(button:4) == false)
        assert(!hold.scroll(hasVerticalMotion:true))
        hold.begin(button:3); hold.cancel()
        assert(hold.release(button:3) == nil) // pause/reconfigure cancels delayed clicks
        let app = ApplicationMetadata.read(path:"/System/Applications/Calculator.app")
        assert(app["available"] as? Bool == true)
        let icon = app["icon"] as! String
        assert(icon.hasPrefix("data:image/png;base64,"))
        let bytes = Data(base64Encoded:String(icon.dropFirst("data:image/png;base64,".count)))!
        let bitmap = NSBitmapImageRep(data:bytes)!
        assert(bitmap.pixelsWide == 64 && bitmap.pixelsHigh == 64)
        let absent = ApplicationMetadata.read(path:"/Applications/LogiLiteMissingTest.app")
        assert(absent["available"] as? Bool == false && absent["name"] as? String == "LogiLiteMissingTest")
        assert(absent["icon"] as? String == "")
        assert(ApplicationMetadata.read(path:"/System/Applications/Calculator.app/Contents/Info.plist")["available"] as? Bool == false)
        // Construct events only: no post(), permissions, actions or user config changes.
        let wheel = CGEvent(scrollWheelEvent2Source:nil, units:.pixel, wheelCount:2, wheel1:3, wheel2:2, wheel3:0)!
        wheel.setDoubleValueField(.scrollWheelEventFixedPtDeltaAxis1, value:0.125)
        wheel.setDoubleValueField(.scrollWheelEventFixedPtDeltaAxis2, value:0.25)
        wheel.setIntegerValueField(.scrollWheelEventPointDeltaAxis1, value:5)
        wheel.setIntegerValueField(.scrollWheelEventPointDeltaAxis2, value:2)
        wheel.setIntegerValueField(.scrollWheelEventScrollPhase, value:2)
        wheel.flags = [.maskShift]
        let expectedLine = wheel.getIntegerValueField(.scrollWheelEventDeltaAxis1) + wheel.getIntegerValueField(.scrollWheelEventDeltaAxis2)
        let expectedFixed = wheel.getDoubleValueField(.scrollWheelEventFixedPtDeltaAxis1) + wheel.getDoubleValueField(.scrollWheelEventFixedPtDeltaAxis2)
        let expectedPoint = wheel.getIntegerValueField(.scrollWheelEventPointDeltaAxis1) + wheel.getIntegerValueField(.scrollWheelEventPointDeltaAxis2)
        HorizontalWheel.convert(wheel)
        assert(wheel.getIntegerValueField(.scrollWheelEventDeltaAxis1) == 0)
        assert(wheel.getIntegerValueField(.scrollWheelEventDeltaAxis2) == expectedLine)
        assert(wheel.getIntegerValueField(.scrollWheelEventFixedPtDeltaAxis1) == 0)
        assert(wheel.getDoubleValueField(.scrollWheelEventFixedPtDeltaAxis2) == expectedFixed)
        assert(wheel.getIntegerValueField(.scrollWheelEventPointDeltaAxis1) == 0)
        assert(wheel.getIntegerValueField(.scrollWheelEventPointDeltaAxis2) == expectedPoint)
        assert(wheel.getIntegerValueField(.scrollWheelEventScrollPhase) == 2)
        assert(wheel.flags == [.maskShift])
        for media in MediaAction.allCases {
            guard case .media(let parsed) = try ShortcutParser.parse(media.rawValue), parsed == media,
                  let down = media.event(down:true, marker:123), let up = media.event(down:false, marker:123),
                  let nativeDown = NSEvent(cgEvent:down), let nativeUp = NSEvent(cgEvent:up) else {
                fatalError("Media actions must parse and create balanced system events")
            }
            assert(nativeDown.subtype.rawValue == 8)
            assert(nativeDown.data1 == (media.key << 16) | (0xa << 8))
            assert(nativeUp.data1 == (media.key << 16) | (0xb << 8))
            assert(down.getIntegerValueField(.eventSourceUserData) == 123)
        }
        for category in ActionCatalog.categories {
            for action in category["actions"] as! [[String:String]] { _ = try ShortcutParser.parse(action["value"]!) }
        }
        // Removed apps must not invalidate the whole mouse configuration.
        guard case .openApplication(let appURL) = try ShortcutParser.parse("打开应用:/Applications/LogiLiteMissingTest.app"),
              appURL.path == "/Applications/LogiLiteMissingTest.app" else {
            fatalError("app action parser is wrong")
        }
        for action in SystemAction.allCases {
            guard case .system(let parsed) = try ShortcutParser.parse(action.rawValue), parsed == action else {
                fatalError("Built-in system action must not become a shortcut")
            }
            // Launch Services requires access to the logged-in GUI session.
            if CommandLine.arguments.contains("--resolve-system-actions"),
               action.applicationURL().map({ FileManager.default.fileExists(atPath:$0.path) }) != true {
                fatalError("System action application could not be resolved: \(action.rawValue)")
            }
        }
        var pairs = ButtonPairs()
        assert(pairs.down(button:3, shouldSuppress:true))
        // After runtime pause or config change the drag and up still match the swallowed down.
        assert(pairs.drag(button:3))
        assert(!pairs.drag(button:4))
        assert(!pairs.up(button:4))
        assert(pairs.up(button:3))
        assert(!pairs.drag(button:3))
        assert(!pairs.up(button:3))
        assert(!pairs.down(button:2, shouldSuppress:false))
        assert(!pairs.up(button:2))
        assert(pairs.swallowed.isEmpty)
        print("Input parser and button pairing tests passed")
    }

}
