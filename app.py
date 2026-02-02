from dataclasses import dataclass

import wit_world


@dataclass
class AddResult:
    length: int
    data: bytes


class WitWorld(wit_world.WitWorld):
    def add(self, data: bytes) -> AddResult:
        return AddResult(length=len(data), data=bytes(data))