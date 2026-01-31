# Bitwise Approval System

## Setup
```rust
pub approval_bitmap: u8  // 8-bit number (can hold values(number) 0-255)
const MAX_OWNERS: usize = 8;  // Maximum 8 owners
```

---

## What is a Bit?

A **bit** is the smallest unit of data in a computer. It can only be **0** or **1**.

A `u8` (unsigned 8-bit integer) is made of **8 bits** arranged in a row:

```
Position:  7  6  5  4  3  2  1  0
Bit:      [0][0][0][0][0][0][0][0]
```

Each position represents an owner (0 through 7).

---

## Where Decimal Numbers Come From

Computers convert binary (bits) to decimal using **powers of 2**:

```
Position:     7      6      5      4      3      2      1      0
Power of 2:  2^7    2^6    2^5    2^4    2^3    2^2    2^1    2^0
Value:       128     64     32     16      8      4      2      1
```

**To get decimal**: Add up the values where the bit is `1`

### Example:
```
Binary:     [0][0][0][0][0][1][0][1]
Values:      0   0   0   0   0   4   0   1
Decimal:    0 + 0 + 0 + 0 + 0 + 4 + 0 + 1 = 5
```

---

## Initial State

```rust
approval_bitmap = 0
approval_count = 0
```

### Memory View:
```
Position:  7  6  5  4  3  2  1  0
Bit:      [0][0][0][0][0][0][0][0]
Owner:     7  6  5  4  3  2  1  0
Status:    _  _  _  _  _  _  _  _

Binary: 00000000
Decimal: 0
```

---

## Owner 0 Approves

### Code:
```rust
self.approval_bitmap |= 1u8 << 0;
```

### Step 1: Create the mask
```rust
1u8 = 00000001  // The number 1 in binary

1u8 << 0 means "shift left by 0 positions" (no shift)
```

**Before shift:**
```
Position:  7  6  5  4  3  2  1  0
Bit:      [0][0][0][0][0][0][0][1]
```

**After shift (no change):**
```
Position:  7  6  5  4  3  2  1  0
Bit:      [0][0][0][0][0][0][0][1]

Binary: 00000001
Decimal: 1 (because position 0 = 2^0 = 1)
```

### Step 2: OR operation (`|=`)
```
Current bitmap: 00000000 (0)
Mask:           00000001 (1)
               ----------
Result:         00000001 (1)
```

**How OR works:** If **either** bit is 1, result is 1.
```
0 | 0 = 0
0 | 1 = 1  ← this happened at position 0
1 | 0 = 1
1 | 1 = 1
```

### Result:
```
Position:  7  6  5  4  3  2  1  0
Bit:      [0][0][0][0][0][0][0][1]
Owner:     7  6  5  4  3  2  1  0
Status:    _  _  _  _  _  _  _  ✓

approval_bitmap = 1
approval_count = 1
Binary: 00000001
Decimal: 1
```

---

## Owner 1 Approves

### Code:
```rust
self.approval_bitmap |= 1u8 << 1;
```

### Step 1: Create the mask
```rust
1u8 = 00000001

1u8 << 1 means "shift left by 1 position"
```

**Before shift:**
```
Position:  7  6  5  4  3  2  1  0
Bit:      [0][0][0][0][0][0][0][1]
```

**After shift left by 1:**
```
Position:  7  6  5  4  3  2  1  0
Bit:      [0][0][0][0][0][0][1][0]
          ↑ new bit fills with 0

Binary: 00000010
Decimal: 2 (because position 1 = 2^1 = 2)
```

### Step 2: OR operation
```
Current bitmap: 00000001 (1)
Mask:           00000010 (2)
               ----------
Result:         00000011 (3)
```

**Bit-by-bit:**
```
Position 0: 1 | 0 = 1
Position 1: 0 | 1 = 1
Position 2-7: 0 | 0 = 0
```

### Result:
```
Position:  7  6  5  4  3  2  1  0
Bit:      [0][0][0][0][0][0][1][1]
Owner:     7  6  5  4  3  2  1  0
Status:    _  _  _  _  _  _  ✓  ✓

approval_bitmap = 3
approval_count = 2
Binary: 00000011
Decimal: 3 (position 0 + position 1 = 1 + 2 = 3)
```

---

## Owner 2 Approves

### Step 1: Create mask
```rust
1u8 << 2  // Shift left by 2 positions

Before: 00000001
After:  00000100

Decimal: 4 (position 2 = 2^2 = 4)
```

### Step 2: OR operation
```
Current: 00000011 (3)
Mask:    00000100 (4)
        ----------
Result:  00000111 (7)
```

### Result:
```
Position:  7  6  5  4  3  2  1  0
Bit:      [0][0][0][0][0][1][1][1]
Owner:     7  6  5  4  3  2  1  0
Status:    _  _  _  _  _  ✓  ✓  ✓

approval_bitmap = 7
approval_count = 3
Decimal: 7 (4 + 2 + 1)
```

---

## Owner 5 Approves

### Step 1: Create mask
```rust
1u8 << 5

Before: 00000001
After:  00100000

Decimal: 32 (position 5 = 2^5 = 32)
```

### Step 2: OR operation
```
Current: 00011111 (31)
Mask:    00100000 (32)
        ----------
Result:  00111111 (63)
```

### Result:
```
Position:  7  6  5  4  3  2  1  0
Bit:      [0][0][1][1][1][1][1][1]
Owner:     7  6  5  4  3  2  1  0
Status:    _  _  ✓  ✓  ✓  ✓  ✓  ✓

approval_bitmap = 63
approval_count = 6
Decimal: 63 (32 + 16 + 8 + 4 + 2 + 1)
```

---

## Owner 7 Approves

### Step 1: Create mask
```rust
1u8 << 7

Before: 00000001
After:  10000000

Decimal: 128 (position 7 = 2^7 = 128)
```

### Step 2: OR operation
```
Current: 01111111 (127)
Mask:    10000000 (128)
        ----------
Result:  11111111 (255)
```

### Result:
```
Position:  7  6  5  4  3  2  1  0
Bit:      [1][1][1][1][1][1][1][1]
Owner:     7  6  5  4  3  2  1  0
Status:    ✓  ✓  ✓  ✓  ✓  ✓  ✓  ✓

approval_bitmap = 255
approval_count = 8
Decimal: 255 (128 + 64 + 32 + 16 + 8 + 4 + 2 + 1)
```

---

## How `has_approved()` Works

```rust
pub fn has_approved(&self, owner_index: usize) -> bool {
    (self.approval_bitmap & (1u8 << owner_index)) != 0
}
```

### Example: Check if Owner 5 approved when bitmap = 63

```
approval_bitmap = 63 = 00111111
```

### Step 1: Create mask for Owner 5
```rust
1u8 << 5 = 00100000 (32)
```

### Step 2: AND operation (`&`)
```
Bitmap: 00111111 (63)
Mask:   00100000 (32)
       ----------
Result: 00100000 (32)
```

**How AND works:** **Both** bits must be 1 for result to be 1.
```
Position 5: 1 & 1 = 1  ← Owner 5's bit is set!
All others: either 0 & 1 = 0 or 1 & 0 = 0
```

### Step 3: Check if non-zero
```rust
32 != 0  →  true  // Owner 5 HAS approved
```

---

### Example: Check if Owner 7 approved (they haven't)

```
approval_bitmap = 63 = 00111111
```

### Step 1: Create mask
```rust
1u8 << 7 = 10000000 (128)
```

### Step 2: AND operation
```
Bitmap: 00111111 (63)
Mask:   10000000 (128)
       ----------
Result: 00000000 (0)
```

**Position 7:** `0 & 1 = 0` (Owner 7's bit is NOT set)

### Step 3: Check
```rust
0 != 0  →  false  // Owner 7 has NOT approved
```

---