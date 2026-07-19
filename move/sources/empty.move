module forum::empty;

public struct Empty has drop, store {}

public fun new(): Empty { Empty {} }
