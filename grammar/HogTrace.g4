grammar HogTrace;

// ===== Parser Rules =====

// Top-level: one or more probe definitions
program
    : probe+ EOF
    ;

// Probe definition: spec, optional predicate, action block
probe
    : probeSpec predicate? action
    ;

// Probe specification: provider:module.function:probe-point
probeSpec
    : provider=PROVIDER ':' moduleFunction ':' probePoint
    ;

// Module and function path with dots
moduleFunction
    : IDENTIFIER ('.' (IDENTIFIER | WILDCARD))*
    ;

// Probe points: entry, exit, or with line offsets
probePoint
    : 'entry'                           # EntryProbe
    | 'exit'                            # ExitProbe
    | 'entry' '+' offset=INT            # EntryOffsetProbe
    | 'exit' '+' offset=INT             # ExitOffsetProbe
    ;

// Predicate (guard condition)
predicate
    : '/' expression '/'
    ;

// Action block with statements
action
    : '{' statement* '}'
    ;

// Statements in action block
statement
    : assignment
    | sampleDirective
    | captureStatement
    ;

// Assignment to request-scoped variable
assignment
    : requestVar '=' expression ';'
    ;

// Request-scoped variable reference
requestVar
    : '$req' '.' IDENTIFIER
    | '$request' '.' IDENTIFIER
    ;

// Sample directive
sampleDirective
    : 'sample' sampleSpec ';'
    ;

sampleSpec
    : INT '%'                           # PercentageSample
    | INT '/' INT                       # RatioSample
    ;

// Capture/send statement
captureStatement
    : ('capture' | 'send') '(' captureArgs? ')' ';'
    ;

// Arguments to capture: either expressions or named arguments
captureArgs
    : namedArg (',' namedArg)*          # NamedCaptureArgs
    | expression (',' expression)*      # PositionalCaptureArgs
    ;

namedArg
    : IDENTIFIER '=' expression
    ;

// ===== Expressions =====

expression
    : expression '.' IDENTIFIER                                     # FieldAccess
    | expression '[' expression ']'                                 # IndexAccess
    | IDENTIFIER '(' expressionList? ')'                            # FunctionCall
    | requestVar                                                    # RequestVarExpr
    | IDENTIFIER                                                    # IdentifierExpr
    | literal                                                       # LiteralExpr
    | '(' expression ')'                                            # ParenExpr
    | '!' expression                                                # NotExpr
    | expression op=('*'|'/'|'%') expression                        # MulDivModExpr
    | expression op=('+'|'-') expression                            # AddSubExpr
    | expression op=('<'|'>'|'<='|'>=') expression                  # ComparisonExpr
    | expression op=('=='|'!=') expression                          # EqualityExpr
    | expression op='&&' expression                                 # AndExpr
    | expression op='||' expression                                 # OrExpr
    ;

expressionList
    : expression (',' expression)*
    ;

// Literals
literal
    : INT                               # IntLiteral
    | FLOAT                             # FloatLiteral
    | STRING                            # StringLiteral
    | BOOL                              # BoolLiteral
    | NONE                              # NoneLiteral
    ;

// ===== Lexer Rules =====

// Keywords
PROVIDER
    : 'fn'
    | 'py'
    ;

BOOL
    : 'True'
    | 'False'
    ;

NONE
    : 'None'
    ;

// Literals
INT
    : [0-9]+
    ;

FLOAT
    : [0-9]+ '.' [0-9]+
    | [0-9]+ '.' [0-9]+ ('e'|'E') ('+'|'-')? [0-9]+
    | [0-9]+ ('e'|'E') ('+'|'-')? [0-9]+
    ;

STRING
    : '"' (~["\r\n\\] | '\\' .)* '"'
    | '\'' (~['\r\n\\] | '\\' .)* '\''
    ;

WILDCARD
    : '*'
    ;

// Identifiers
IDENTIFIER
    : [a-zA-Z_][a-zA-Z0-9_]*
    ;

// Whitespace and comments
WS
    : [ \t\r\n]+ -> skip
    ;

COMMENT
    : '#' ~[\r\n]* -> skip
    ;

BLOCK_COMMENT
    : '/*' .*? '*/' -> skip
    ;
