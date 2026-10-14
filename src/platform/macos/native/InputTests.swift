import Foundation
import CoreGraphics

@main
enum InputTests {
    static func main() throws {
        for text in ["默认", "禁用", "Cmd+C", "Command+Shift+Z", "Option+Left", "ctrl + 2", "F20"] {
            _ = try ShortcutParser.parse(text)
        }
        for text in ["", "Cmd", "Cmd+", "Cmd+Command+C", "Alt+Option+C", "Ctrl+C+V", "F21", "Ctrl++", "手势按钮"] {
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
