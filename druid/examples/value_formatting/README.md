# Validation

This example demonstrates how to add a formatter to a textbox in order to ensure
that the textbox contains valid data.

The example is reasonably complex, for a number of reasons: firstly there are
currently no built-in implementations of the `Formatter` trait included in
Druid, which means we have to write versions for the example, and additionally
the mechanism for reporting errors that occur during formatting are difficult to
handle in Druid's current architecture, and involve some fairly ugly use of
`Command` in order to function.
