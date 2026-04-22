use std/testing *

@before-each
def something-before-each [] {
    highlight-print "something-before-each"
}

@after-each
def wow-after-each [] {
    highlight-print "wow-after-each"
}

@before-all
def and-before-all [] {
    highlight-print "and-before-all"
}

@after-all
def something-at-the-end [] {
    highlight-print "something-at-the-end"
}

@test
def something-to-test [] {
    highlight-print "something-to-test"
}

@test
def another-test [] {
    highlight-print "another-test"
}

def highlight-print [content: string] {
    print $"  (ansi yellow)($content)(ansi reset)"
}
