#!/usr/bin/env python3
"""Independent recursive-descent/AST calculator reference. Never uses eval.

The entire lexer and parser finish before checked AST evaluation. Object pairs
and number tokens are preserved so duplicate/nonintegral selected values remain
observable. This file and all rung cases are frozen before the AIL implementation.
"""
from __future__ import annotations

import json
import re
import sys

MINIMUM = -(1 << 63)
MAXIMUM = (1 << 63) - 1
WHITESPACE = ' \t\n\r\v\f'
# A bounded expression can contain 255 nested parentheses; recursive descent has
# several Python frames per grammar level. This is a reference implementation
# detail, not a relaxation of the 512-token language bound.
sys.setrecursionlimit(max(sys.getrecursionlimit(), 10000))


class CalcError(Exception):
    pass


class Object(list):
    """JSON object pairs, deliberately distinct from JSON arrays."""


class Number(str):
    """Lossless JSON numeric spelling."""


def reject_constant(_):
    raise ValueError('non-JSON constant')


def parse_json(text):
    value = json.loads(text, object_pairs_hook=Object, parse_int=Number,
                       parse_float=Number, parse_constant=reject_constant)
    def validate(item):
        if isinstance(item, str):
            item.encode('utf-8')
        elif isinstance(item, Object):
            for key, child in item:
                key.encode('utf-8')
                validate(child)
        elif isinstance(item, list):
            for child in item:
                validate(child)
    validate(value)
    return value


def tokens(expression):
    result = []
    position = 0
    while position < len(expression):
        char = expression[position]
        if char in WHITESPACE:
            position += 1
            continue
        start = position
        if '0' <= char <= '9':
            position += 1
            while position < len(expression) and '0' <= expression[position] <= '9':
                position += 1
            token = ('number', expression[start:position])
        elif char == '_' or 'a' <= char <= 'z' or 'A' <= char <= 'Z':
            position += 1
            while position < len(expression):
                char = expression[position]
                if char == '_' or 'a' <= char <= 'z' or 'A' <= char <= 'Z' or '0' <= char <= '9':
                    position += 1
                else:
                    break
            token = ('name', expression[start:position])
        elif char in '+-*/%()':
            position += 1
            token = (char, char)
        else:
            raise CalcError('LEX')
        result.append(token)
        if len(result) > 512:
            raise CalcError('LIMIT')
    return result


class Parser:
    def __init__(self, source):
        self.tokens = source
        self.position = 0

    def peek(self):
        return self.tokens[self.position][0] if self.position < len(self.tokens) else None

    def take(self):
        token = self.tokens[self.position]
        self.position += 1
        return token

    def parse(self):
        tree = self.sum()
        if self.peek() is not None:
            raise CalcError('SYNTAX')
        return tree

    def sum(self):
        result = self.product()
        while self.peek() in ('+', '-'):
            operator = self.take()[0]
            result = ('binary', operator, result, self.product())
        return result

    def product(self):
        result = self.unary()
        while self.peek() in ('*', '/', '%'):
            operator = self.take()[0]
            result = ('binary', operator, result, self.unary())
        return result

    def unary(self):
        if self.peek() in ('+', '-'):
            operator = self.take()[0]
            return ('unary', operator, self.unary())
        return self.primary()

    def primary(self):
        kind = self.peek()
        if kind in ('number', 'name'):
            return self.take()
        if kind == '(':
            self.take()
            result = self.sum()
            if self.peek() != ')':
                raise CalcError('SYNTAX')
            self.take()
            return result
        raise CalcError('SYNTAX')


def checked(value):
    if value < MINIMUM or value > MAXIMUM:
        raise CalcError('OVERFLOW')
    return value


def decimal(text, code, signed=False):
    if signed and text.startswith('-'):
        negative, digits = True, text[1:]
    else:
        negative, digits = False, text
    significant = digits.lstrip('0') or '0'
    bound = str(-MINIMUM if negative else MAXIMUM)
    if len(significant) > len(bound) or (len(significant) == len(bound) and significant > bound):
        raise CalcError(code)
    return -int(significant) if negative else int(significant)


def variable(variables, name):
    matches = [value for key, value in variables if key == name]
    if len(matches) != 1 or not isinstance(matches[0], Number):
        raise CalcError('NAME')
    token = matches[0]
    if re.fullmatch(r'-?(?:0|[1-9][0-9]*)', token) is None:
        raise CalcError('NAME')
    return decimal(token, 'NAME', signed=True)


def evaluate_tree(tree, variables):
    kind = tree[0]
    if kind == 'number':
        return decimal(tree[1], 'OVERFLOW')
    if kind == 'name':
        return variable(variables, tree[1])
    if kind == 'unary':
        value = evaluate_tree(tree[2], variables)
        return checked(-value) if tree[1] == '-' else value
    left = evaluate_tree(tree[2], variables)
    right = evaluate_tree(tree[3], variables)
    operator = tree[1]
    if operator == '+':
        return checked(left + right)
    if operator == '-':
        return checked(left - right)
    if operator == '*':
        return checked(left * right)
    if right == 0:
        raise CalcError('DIVZERO')
    if left == MINIMUM and right == -1:
        raise CalcError('OVERFLOW')
    quotient = abs(left) // abs(right) * (-1 if (left < 0) != (right < 0) else 1)
    return quotient if operator == '/' else left - quotient * right


def failure(code):
    return {'ok': False, 'value': 0, 'error': code}


def evaluate(expression, variables):
    if not isinstance(variables, Object) or not isinstance(expression, str):
        return failure('REQUEST')
    if len(expression) > 4096:
        return failure('LIMIT')
    try:
        tree = Parser(tokens(expression)).parse()
        return {'ok': True, 'value': evaluate_tree(tree, variables), 'error': ''}
    except CalcError as error:
        return failure(str(error))


def evaluate_request(text):
    try:
        request = parse_json(text)
        if not isinstance(request, Object):
            return failure('REQUEST')
        expressions = [value for key, value in request if key == 'expression']
        variables = [value for key, value in request if key == 'variables']
        if len(expressions) != 1 or type(expressions[0]) is not str or len(variables) != 1 or not isinstance(variables[0], Object):
            return failure('REQUEST')
        return evaluate(expressions[0], variables[0])
    except (ValueError, UnicodeError):
        return failure('REQUEST')


def encode(result):
    return json.dumps(result, ensure_ascii=False, separators=(',', ':'))


def main(argv):
    state = dict(argument.split('=', 1) for argument in argv if '=' in argument)
    variables = parse_json(state['data.variables'])
    print('out.result=' + encode(evaluate(state['data.expression'], variables)))
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
