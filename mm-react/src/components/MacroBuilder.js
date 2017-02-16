//! Macro builder component.  Allows users to visually create spawner macros and edit spawner macros that they
//! have already created.

import React from 'react';
import { connect } from 'dva';
import { Select } from 'antd';

const MacroBuilder = ({dispatch}) => {
  return (
    <Select>
      {/*TODO*/}
    </Select>
  );
};

MacroBuilder.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
};

export default connect()(MacroBuilder);
