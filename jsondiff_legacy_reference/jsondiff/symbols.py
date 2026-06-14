class Symbol:
    """
    Symbol Usage Explanation:

    $add: Indicates elements added to a set.
    $discard: Indicates elements removed from a set.
    $delete: Indicates keys (dict) or positions (list) where elements were removed.
    $insert: Indicates new elements inserted into a list at specific positions.
    $replace: Indicates the value at a given location is wholly replaced.

    These symbols are used within the diff structures returned by JsonDiffer to
    represent different types of changes between two JSON structures.
    """
    def __init__(self, label):
        self._label = label

    @property
    def label(self):
        return self._label

    def __repr__(self):
        return self.label

    def __str__(self):
        return "$" + self.label

    def __eq__(self, other):
        if not isinstance(other, Symbol):
            return False
        return self.label == other.label

    def __hash__(self) -> int:
        return hash(self.label)


missing = Symbol('missing')
delete = Symbol('delete')
insert = Symbol('insert')
add = Symbol('add')
discard = Symbol('discard')
replace = Symbol('replace')

_all_symbols_ = [
    missing,
    delete,
    insert,
    add,
    discard,
    replace,
]

__all__ = [
    'missing',
    'delete',
    'insert',
    'add',
    'discard',
    'replace',
    '_all_symbols_'
]
