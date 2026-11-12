/// Fixed-capacity history; appending past capacity drops the oldest value.
public struct RingBuffer<Element>: Sendable where Element: Sendable {
    public let capacity: Int
    private var storage: [Element] = []
    private var head = 0

    public init(capacity: Int) {
        precondition(capacity > 0, "capacity must be positive")
        self.capacity = capacity
        storage.reserveCapacity(capacity)
    }

    public mutating func append(_ element: Element) {
        if storage.count < capacity {
            storage.append(element)
        } else {
            storage[head] = element
            head = (head + 1) % capacity
        }
    }

    /// Oldest first.
    public var elements: [Element] {
        Array(storage[head...] + storage[..<head])
    }

    public var last: Element? {
        guard !storage.isEmpty else { return nil }
        return storage[(head + storage.count - 1) % storage.count]
    }

    public var count: Int { storage.count }
}
