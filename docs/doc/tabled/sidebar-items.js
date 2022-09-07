window.SIDEBAR_ITEMS = {"derive":[["Tabled",""]],"enum":[["Alignment","Alignment represent a horizontal and vertical alignemt setting for any cell on a [crate::Table]."],["AlignmentHorizontal","AlignmentHorizontal represents an horizontal aligment of a cell content."],["AlignmentVertical","AlignmentVertical represents an vertical aligment of a cell content."],["Disable","Disable removes particular rows/columns from a [Table]."],["Rotate","Rotate can be used to rotate a table by 90 degrees."],["Target",""]],"fn":[["multiline","Multiline a helper function for changing multiline content of cell. Using this formatting applied for all rows not to a string as a whole."]],"mod":[["builder","Builder module provides a [Builder] type which helps building a [Table] dynamically."],["display",""],["style","This module contains a list of Styles which can be applied to change [Table] styles."]],"struct":[["Border",""],["Cell","Cell denotes a particular cell on a [Grid]."],["Column","Column denotes a set of cells on given columns on a [Grid]."],["Combination","Combination struct used for chaning [Object]’s."],["Concat","Concat concatenate tables along a particular axis [Horizontal | Vertical]. It doesn’t do any key or column comparisions like SQL’s join does."],["Footer","Footer renders a [Panel] at the bottom. See [Panel]."],["Format","Formatting of particular cells on a [Grid]."],["FormatFrom","FormatFrom repeatedly uses first possible element from given array unless there’s any elements."],["FormatWithIndex","FormatWithIndex is like a [Format]. But it also provides a row and column index."],["Full","Full represents all cells on a [Grid]"],["Head","Head represents the row at the top of a [Table]."],["Header","Header inserts a [Panel] at the top. See [Panel]."],["Highlight",""],["Indent","Indent is responsible for a left/right/top/bottom indent of particular cells."],["MaxWidth","MaxWidth allows you to set a max width of an object on a [Grid], using different strategies."],["Modify","Modify structure provide an abstraction, to be able to apply a set of [CellOption]s to the same object."],["Panel","Panel allows to add a Row which has 1 continues Cell to a [Table]."],["Row","Row denotes a set of cells on given rows on a [Grid]."],["Table","Table structure provides an interface for building a table for types that implements [Tabled]."],["Truncate","Truncate cut the string to a given width if its length exeeds it. Otherwise keeps the content of a cell untouched."],["Wrap","Wrap wraps a string to a new line in case it exeeds the provided max boundry. Otherwise keeps the content of a cell untouched."]],"trait":[["CellOption","A trait for configuring a [Cell] a single cell. Where cell represented by ‘row’ and ‘column’ indexes."],["Object","Object helps to locate a nessesary part of a [Grid]."],["TableIteratorExt","A trait for [IntoIterator] whose Item type is bound to [Tabled]. Any type implements [IntoIterator] can call this function directly"],["TableOption","A trait which is responsilbe for configuration of a [Grid]."],["Tabled","Tabled a trait responsible for providing a header fields and a row fields."]]};