import XCTest
import SwiftTreeSitter
import TreeSitterCivicc

final class TreeSitterCiviccTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_civicc())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading civicc grammar")
    }
}
