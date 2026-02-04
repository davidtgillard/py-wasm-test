import wit_world


class WitWorld(wit_world.WitWorld):
    def add(self, data: bytes) -> wit_world.AddResultOrError:
        if len(data) == 0:
            return wit_world.AddResultOrError_Err(value="empty input")
        return wit_world.AddResultOrError_Ok(
            value=wit_world.AddResult(length=len(data), data=bytes(data))
        )
