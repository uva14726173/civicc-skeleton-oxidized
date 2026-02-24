/**
 * @file Civicc grammar for tree-sitter
 * @author YOUR NAME HERE
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "civicc",

  extras: ($) => [
    /\s/, // whitespace
    $.comment,
  ],

  supertypes: $ => [
    $.expr,
  ],

  rules: {
    source_file: $ => repeat($.assign),
    assign: $ => seq(field("varlet", $.id), "=", field("expr", $.expr), ";"),
    expr: $ => choice($.constant, $.id, $.dyop),
    dyop: $ => seq("(", field("left", $.expr), field("op", $.dyoptype), field("right", $.expr), ")"),

    dyoptype: $ => token(choice("+", "-", "*", "/", "<=", "<", ">=", ">", "==", "!=", "&&", "||")),
    boolval: $ => token(choice("true","false")),
    constant: $ => choice($.floatval, $.intval, $.boolval),
    floatval: $ => /[1-9][0-9]*\.[0-9]+/,
    intval: $ => /[0-9]+/,
    id: $ => /[A-Za-z][A-Za-z0-9_]*/,

    comment: $ => token(choice(seq("//", /.*/), seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/"))),
  }
});
