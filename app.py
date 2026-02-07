import length_calc


class LengthCalc(length_calc.LengthCalc):
    def length_calc(self, data: bytes) -> length_calc.LengthCalcResultOrError:
        if len(data) == 0:
            return length_calc.LengthCalcResultOrError_Err(value="empty input")
        return length_calc.LengthCalcResultOrError_Ok(
            value=length_calc.LengthCalcResult(length=len(data), data=bytes(data))
        )
