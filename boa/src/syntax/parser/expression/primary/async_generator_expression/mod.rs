//! Async Generator Expression Parser
//!
//! Implements TokenParser for AsyncGeneratorExpression and outputs
//! an AsyncGeneratorExpr ast node
//!
//!  More information:
//!  - [ECMAScript specification][spec]
//!
//! [spec]: https://tc39.es/ecma262/#prod-AsyncGeneratorExpression
#[cfg(test)]
mod test;

use crate::{
    syntax::{
        ast::{node::AsyncGeneratorExpr, Keyword, Punctuator},
        lexer::{Error as LexError, Position, TokenKind},
        parser::{
            function::{FormalParameters, FunctionBody},
            statement::BindingIdentifier,
            Cursor, ParseError, TokenParser,
        },
    },
    BoaProfiler,
};

use std::io::Read;

/// Async Generator Expression Parsing
///
/// More information:
///  - [ECMAScript specification][spec]
///
/// [spec]: https://tc39.es/ecma262/#prod-AsyncGeneratorExpression
#[derive(Debug, Clone, Copy)]
pub(super) struct AsyncGeneratorExpression;

impl<R> TokenParser<R> for AsyncGeneratorExpression
where
    R: Read,
{
    //The below needs to be implemented in ast::node
    type Output = AsyncGeneratorExpr;

    fn parse(self, cursor: &mut Cursor<R>) -> Result<Self::Output, ParseError> {
        let _timer = BoaProfiler::global().start_event("AsyncGeneratorExpression", "Parsing");

        cursor.peek_expect_no_lineterminator(0, "async generator expression")?;
        cursor.expect(Keyword::Function, "async generator expression")?;
        cursor.expect(
            TokenKind::Punctuator(Punctuator::Mul),
            "async generator expression",
        )?;

        let name = if let Some(token) = cursor.peek(0)? {
            match token.kind() {
                TokenKind::Punctuator(Punctuator::OpenParen) => None,
                _ => Some(BindingIdentifier::new(true, true).parse(cursor)?),
            }
        } else {
            return Err(ParseError::AbruptEnd);
        };

        // Early Error: If BindingIdentifier is present and the source code matching BindingIdentifier is strict
        // mode code, it is a Syntax Error if the StringValue of BindingIdentifier is "eval" or "arguments".
        if let Some(name) = &name {
            if cursor.strict_mode() && ["eval", "arguments"].contains(&name.as_ref()) {
                return Err(ParseError::lex(LexError::Syntax(
                    "Unexpected eval or arguments in strict mode".into(),
                    match cursor.peek(0)? {
                        Some(token) => token.span().end(),
                        None => Position::new(1, 1),
                    },
                )));
            }
        }

        let params_start_position = cursor
            .expect(Punctuator::OpenParen, "async generator expression")?
            .span()
            .end();

        let params = FormalParameters::new(true, true).parse(cursor)?;

        cursor.expect(Punctuator::CloseParen, "async generator expression")?;
        cursor.expect(Punctuator::OpenBlock, "async generator expression")?;

        let body = FunctionBody::new(true, true).parse(cursor)?;

        cursor.expect(Punctuator::CloseBlock, "async generator expression")?;

        // Early Error: If the source code matching FormalParameters is strict mode code,
        // the Early Error rules for UniqueFormalParameters : FormalParameters are applied.
        if (cursor.strict_mode() || body.strict()) && params.has_duplicates {
            return Err(ParseError::lex(LexError::Syntax(
                "Duplicate parameter name not allowed in this context".into(),
                params_start_position,
            )));
        }

        // Early Error: It is a Syntax Error if FunctionBodyContainsUseStrict of GeneratorBody is true
        // and IsSimpleParameterList of FormalParameters is false.
        if body.strict() && !params.is_simple {
            return Err(ParseError::lex(LexError::Syntax(
                "Illegal 'use strict' directive in function with non-simple parameter list".into(),
                params_start_position,
            )));
        }

        // It is a Syntax Error if any element of the BoundNames of FormalParameters
        // also occurs in the LexicallyDeclaredNames of FunctionBody.
        {
            let lexically_declared_names = body.lexically_declared_names();
            for param in params.parameters.as_ref() {
                if lexically_declared_names.contains(param.name()) {
                    return Err(ParseError::lex(LexError::Syntax(
                        format!("Redeclaration of formal parameter `{}`", param.name()).into(),
                        match cursor.peek(0)? {
                            Some(token) => token.span().end(),
                            None => Position::new(1, 1),
                        },
                    )));
                }
            }
        }

        //implement the below AsyncGeneratorExpr in ast::node
        Ok(AsyncGeneratorExpr::new(name, params.parameters, body))
    }
}
