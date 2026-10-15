import Foundation
import CoreGraphics
import AppKit

@main
enum InputTests {
    static func main() throws {
        for text in ["默认", "禁用", "水平滚动", "Cmd+C", "Command+Shift+Z", "Option+Left", "ctrl + 2", "F20"] {
            _ = try ShortcutParser.parse(text)
        }
        for text in ["", "Cmd", "Cmd+", "Cmd+Command+C", "Alt+Option+C", "Ctrl+C+V", "F21", "Ctrl++", "手势按钮", "打开应用:NotAnAbsolute.app"] {
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
